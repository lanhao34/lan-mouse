import AppKit
import Foundation
import Darwin

// Separate, small windows: the desktop never has a fullscreen input surface.
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
func fraction(_ point: NSPoint, frame: NSRect, vertical: Bool) -> Double {
    let value = vertical ? (frame.maxY - point.y) / max(1,frame.height-1) : (point.x-frame.minX)/max(1,frame.width-1)
    return min(1,max(0,value))
}
// Endpoint snapping is based on the pointer's real screen position, not the
// inset handle center. Preserve the grab offset everywhere else.
func draggedFraction(initial: Double, delta: NSPoint, pointer: NSPoint, frame: NSRect, vertical: Bool) -> Double {
    let span=max(1,vertical ? frame.height-1 : frame.width-1)
    let axis=vertical ? frame.maxY-pointer.y : pointer.x-frame.minX
    let snap=min(12,span/4)
    if axis<=snap{return 0}
    if axis>=span-snap{return 1}
    return min(1,max(0,initial + (vertical ? -delta.y : delta.x)/span))
}
// AppKit normally constrains windows to visibleFrame, excluding the menu bar.
// Only our tiny edge windows may reach the full display frame.
final class EdgePanel: NSPanel {
    override func constrainFrameRect(_ frameRect: NSRect, to screen: NSScreen?) -> NSRect { frameRect }
}
if CommandLine.arguments.contains("--self-test") {
    let f=NSRect(x:-1920,y:-100,width:1920,height:1080)
    assert(fraction(NSPoint(x:-1920,y:f.maxY),frame:f,vertical:true)==0)
    assert(fraction(NSPoint(x:0,y:f.maxY-1079),frame:f,vertical:true)==1)
    assert(fraction(NSPoint(x:-1920+959.5,y:0),frame:f,vertical:false)==0.5)
    for vertical in [true,false] {
        let begin=NSPoint(x:f.minX,y:f.maxY)
        let end=NSPoint(x:f.maxX-1,y:f.minY+1)
        assert(draggedFraction(initial:0.3,delta:.zero,pointer:begin,frame:f,vertical:vertical)==0)
        assert(draggedFraction(initial:0.7,delta:.zero,pointer:end,frame:f,vertical:vertical)==1)
    }
    assert(draggedFraction(initial:0.5,delta:NSPoint(x:0,y:107.9),pointer:NSPoint(x:0,y:f.midY),frame:f,vertical:true)==0.4)
    let edgePanel=EdgePanel(contentRect:.zero,styleMask:[.borderless],backing:.buffered,defer:false)
    let requested=NSRect(x:-1920,y:956,width:24,height:24)
    assert(edgePanel.constrainFrameRect(requested,to:nil)==requested)
    print("PASS native coordinates, four endpoint snaps, grab offset, unconstrained edge windows");exit(0)
}
let args=Array(CommandLine.arguments.dropFirst())
guard args.count==7,let x=Double(args[0]),let y=Double(args[1]),let w=Double(args[2]),let h=Double(args[3]),let a=Double(args[5]),let b=Double(args[6]),[x,y,w,h,a,b].allSatisfy({$0.isFinite}),a>=0,b<=1,b-a>=0.000999999 else {exit(2)}
let edge=args[4]
guard ["left","right","top","bottom"].contains(edge) else {exit(2)}
let screens=NSScreen.screens
let left=screens.map{$0.frame.minX}.min() ?? 0
let top=screens.map{$0.frame.maxY}.max() ?? 0
let matches=screens.filter { s in let f=s.frame;return abs((f.minX-left)-x)<1 && abs((top-f.maxY)-y)<1 && abs(f.width-w)<1 && abs(f.height-h)<1 }
guard matches.count==1 else { fputs("Monitor geometry no longer matches; reopen the editor.\n",stderr);exit(3) }
let screen=matches[0]
let lockPath=NSTemporaryDirectory()+"lan-mouse-edge-overlay-\(getuid()).lock"
let fd=open(lockPath,O_CREAT|O_RDWR,S_IRUSR|S_IWUSR)
guard fd>=0,flock(fd,LOCK_EX|LOCK_NB)==0 else {fputs("An edge selector is already open.\n",stderr);exit(4)}

final class HandleView: NSView {
    var begin: (() -> Void)?
    var move: ((NSPoint) -> Void)?
    var origin: NSPoint?
    override func draw(_ dirtyRect:NSRect) {
        NSColor.systemCyan.setFill();NSBezierPath(ovalIn:bounds.insetBy(dx:2,dy:2)).fill()
        NSColor.white.setStroke();let circle=NSBezierPath(ovalIn:bounds.insetBy(dx:3,dy:3));circle.lineWidth=2;circle.stroke()
    }
    override func mouseDown(with event:NSEvent){origin=NSEvent.mouseLocation;begin?()}
    override func mouseDragged(with event:NSEvent){if let origin=origin {let p=NSEvent.mouseLocation;move?(NSPoint(x:p.x-origin.x,y:p.y-origin.y))}}
}
final class ControlPanel: NSPanel {
    var cancel: (() -> Void)?
    override var canBecomeKey:Bool { true }
    override func cancelOperation(_ sender:Any?){cancel?()}
}
final class Selector:NSObject {
    let frame:NSRect
    let vertical:Bool
    var start:Double;var end:Double
    let line=EdgePanel(contentRect:.zero,styleMask:[.borderless,.nonactivatingPanel],backing:.buffered,defer:false)
    let first=EdgePanel(contentRect:.zero,styleMask:[.borderless,.nonactivatingPanel],backing:.buffered,defer:false)
    let last=EdgePanel(contentRect:.zero,styleMask:[.borderless,.nonactivatingPanel],backing:.buffered,defer:false)
    let controls=ControlPanel(contentRect:.zero,styleMask:[.titled,.utilityWindow,.nonactivatingPanel],backing:.buffered,defer:false)
    let label=NSTextField(labelWithString:"")
    var done=false
    var observer:NSObjectProtocol?
    var timer:Timer?
    init(frame:NSRect,start:Double,end:Double){self.frame=frame;self.start=start;self.end=end;vertical=edge=="left" || edge=="right";super.init()}
    func point(_ t:Double)->NSPoint {
        switch edge {
        case "left":return NSPoint(x:frame.minX+2,y:frame.maxY-t*(frame.height-1))
        case "right":return NSPoint(x:frame.maxX-3,y:frame.maxY-t*(frame.height-1))
        case "top":return NSPoint(x:frame.minX+t*(frame.width-1),y:frame.maxY-3)
        default:return NSPoint(x:frame.minX+t*(frame.width-1),y:frame.minY+2)
        }
    }
    func show(){
        for p in [line,first,last,controls] {
            p.level = .floating;p.collectionBehavior=[.canJoinAllSpaces,.fullScreenAuxiliary,.stationary]
            p.isReleasedWhenClosed=false;p.hidesOnDeactivate=false
        }
        for p in [line,first,last] { p.level=NSWindow.Level(rawValue:NSWindow.Level.statusBar.rawValue+1) }
        line.isOpaque=false;line.backgroundColor = .systemCyan;line.ignoresMouseEvents=true
        for (index,p) in [first,last].enumerated(){
            p.isOpaque=false;p.backgroundColor = .clear;p.hasShadow=true
            let v=HandleView(frame:NSRect(x:0,y:0,width:24,height:24));p.contentView=v
            var initialValue = 0.0
            v.begin={ [weak self] in guard let self=self else{return};initialValue=index==0 ? self.start : self.end }
            v.move={ [weak self] delta in guard let self=self else{return};let t=draggedFraction(initial:initialValue,delta:delta,pointer:NSEvent.mouseLocation,frame:self.frame,vertical:self.vertical)
                if index==0{self.start=min(t,self.end-0.001)}else{self.end=max(t,self.start+0.001)};self.refresh()
            }
        }
        controls.title="边缘范围 · \(screen.localizedName)"
        controls.setContentSize(NSSize(width:420,height:98))
        controls.setFrameOrigin(NSPoint(x:screen.visibleFrame.midX-210,y:screen.visibleFrame.midY-49))
        let body=NSView(frame:NSRect(x:0,y:0,width:420,height:98))
        label.frame=NSRect(x:16,y:62,width:388,height:22);body.addSubview(label)
        let help=NSTextField(labelWithString:"拖动边缘圆点；桌面正常可用。Esc 取消（面板聚焦时）。")
        help.font = .systemFont(ofSize:11);help.frame=NSRect(x:16,y:40,width:388,height:18);body.addSubview(help)
        let cancel=NSButton(title:"取消",target:self,action:#selector(cancelled));cancel.frame=NSRect(x:232,y:8,width:80,height:28);body.addSubview(cancel)
        let apply=NSButton(title:"确认范围",target:self,action:#selector(accepted));apply.frame=NSRect(x:316,y:8,width:90,height:28);body.addSubview(apply)
        controls.contentView=body;controls.cancel={ [weak self] in self?.cancelled() }
        refresh();for p in [line,first,last]{p.orderFrontRegardless()};controls.makeKeyAndOrderFront(nil)
        observer=NotificationCenter.default.addObserver(forName:NSApplication.didChangeScreenParametersNotification,object:nil,queue:.main){[weak self]_ in self?.cancelled()}
        timer=Timer(timeInterval:300,repeats:false){[weak self]_ in self?.cancelled()};RunLoop.main.add(timer!,forMode:.common)
    }
    func refresh(){
        let p=point(start),q=point(end)
        line.setFrame(vertical ? NSRect(x:p.x-2,y:min(p.y,q.y),width:4,height:max(1,abs(p.y-q.y))) : NSRect(x:min(p.x,q.x),y:p.y-2,width:max(1,abs(p.x-q.x)),height:4),display:true)
        for (panel,point) in [(first,p),(last,q)] {
            panel.setFrame(NSRect(x:min(max(point.x-12,frame.minX),frame.maxX-24),y:min(max(point.y-12,frame.minY),frame.maxY-24),width:24,height:24),display:true)
        }
        label.stringValue=String(format:"起点 %.1f%% → 终点 %.1f%%  ·  %@",start*100,end*100,edge)
    }
    @objc func accepted(){finish(apply:true)}
    @objc func cancelled(){finish(apply:false)}
    func finish(apply:Bool){guard !done else{return};done=true
        if apply {print("{\"start\":\(start),\"end\":\(end)}")}else{print("{\"cancelled\":true}")};fflush(stdout)
        timer?.invalidate();if let observer=observer{NotificationCenter.default.removeObserver(observer)}
        for p in [line,first,last,controls]{p.orderOut(nil)};app.terminate(nil)
    }
}
let selector=Selector(frame:screen.frame,start:a,end:b)
selector.show();app.run()
