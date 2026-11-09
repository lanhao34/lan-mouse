using System.Globalization;
using System.Runtime.InteropServices;
using System.Text.Json;
using Microsoft.Win32;

namespace LanMouse.EdgeOverlay;

static class Geometry
{
    public static bool Matches(Rectangle physical, double[] logical, double scale)
    {
        if (!double.IsFinite(scale) || scale < 0.5 || scale > 8 || logical.Length != 4) return false;
        double[] actual = { physical.X, physical.Y, physical.Width, physical.Height };
        // GDK converts monitor geometry to application pixels; tolerate rounding only.
        return actual.Zip(logical, (p, l) => double.IsFinite(l) && Math.Abs(p - l * scale) <= scale + 1).All(x => x);
    }
    public static double Drag(double initial, Point delta, Point pointer, Rectangle screen, bool vertical)
    {
        double span = Math.Max(1, (vertical ? screen.Height : screen.Width) - 1);
        double axis = vertical ? pointer.Y - screen.Top : pointer.X - screen.Left;
        double snap = Math.Min(12, span / 4);
        if (axis <= snap) return 0;
        if (axis >= span - snap) return 1;
        return Math.Clamp(initial + (vertical ? delta.Y : delta.X) / span, 0, 1);
    }
    public static void Test()
    {
        static void Check(bool condition) { if (!condition) throw new InvalidOperationException("Geometry assertion failed"); }
        var r = new Rectangle(-1920, -100, 1920, 1080);
        Check(Matches(r, new double[] { -960, -50, 960, 540 }, 2));
        Check(Matches(new Rectangle(1920, 0, 2560, 1440), new double[] {1280, 0, 1706, 960}, 1.5));
        Check(!Matches(r, new double[] { 0, 0, 1920, 1080 }, 1));
        Check(!Matches(r, new double[] { 0, 0, 1, 1 }, double.NaN));
        foreach (bool v in new[] { true, false })
        {
            Check(Drag(.3, Point.Empty, new Point(r.Left, r.Top), r, v) == 0);
            Check(Drag(.7, Point.Empty, new Point(r.Right - 1, r.Bottom - 1), r, v) == 1);
        }
        Check(Math.Abs(Drag(.5, new Point(0,108), new Point(-1900,440),r,true) - (.5+108.0/1079)) < 1e-12);
        Console.WriteLine("PASS: DPI matching, negative origins, endpoint snapping, grab offset");
    }
}

sealed class EdgeForm : Form
{
    readonly bool passthrough;
    readonly bool handle;
    public event Action? BeginDrag;
    public event Action<Point,Point>? Dragged;
    Point origin;
    public EdgeForm(bool pass, bool isHandle)
    {
        passthrough = pass; handle = isHandle;
        FormBorderStyle = FormBorderStyle.None; ShowInTaskbar = false; TopMost = true;
        StartPosition = FormStartPosition.Manual; AutoScaleMode = AutoScaleMode.None;
        BackColor = Color.FromArgb(24, 185, 240); DoubleBuffered = true;
        if (isHandle) Cursor = Cursors.SizeAll;
    }
    protected override bool ShowWithoutActivation => true;
    protected override CreateParams CreateParams
    {
        get
        {
            var p = base.CreateParams;
            p.ExStyle |= 0x00000080 | 0x08000000; // TOOLWINDOW | NOACTIVATE
            if (passthrough) p.ExStyle |= 0x00000020 | 0x00080000; // TRANSPARENT | LAYERED
            return p;
        }
    }
    [DllImport("user32.dll")] static extern bool SetLayeredWindowAttributes(IntPtr hwnd, uint key, byte alpha, uint flags);
    protected override void OnHandleCreated(EventArgs e)
    {
        base.OnHandleCreated(e);
        if (passthrough) SetLayeredWindowAttributes(Handle,0,255,2);
    }
    protected override void WndProc(ref Message m)
    {
        if (passthrough && m.Msg == 0x84) { m.Result = new IntPtr(-1); return; } // HTTRANSPARENT
        base.WndProc(ref m);
    }
    protected override void OnResize(EventArgs e)
    {
        base.OnResize(e);
        if (handle && Width > 0 && Height > 0)
        {
            using var shape = new System.Drawing.Drawing2D.GraphicsPath(); shape.AddEllipse(ClientRectangle);
            var old = Region; Region = new Region(shape); old?.Dispose();
        }
    }
    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        if (handle) { using var pen=new Pen(Color.White,2); e.Graphics.DrawEllipse(pen,2,2,Width-5,Height-5); }
    }
    protected override void OnMouseDown(MouseEventArgs e)
    {
        base.OnMouseDown(e); if (e.Button != MouseButtons.Left || !handle) return;
        origin = System.Windows.Forms.Cursor.Position; Capture = true; BeginDrag?.Invoke();
    }
    protected override void OnMouseMove(MouseEventArgs e)
    {
        base.OnMouseMove(e); if (!Capture || !handle) return;
        var p=System.Windows.Forms.Cursor.Position; Dragged?.Invoke(new Point(p.X-origin.X,p.Y-origin.Y),p);
    }
    protected override void OnMouseUp(MouseEventArgs e) { Capture=false; base.OnMouseUp(e); }
}

sealed class Selection : ApplicationContext
{
    readonly Rectangle bounds;
    readonly bool vertical;
    readonly string edge;
    double start, end;
    readonly EdgeForm line = new(true,false), first = new(false,true), last = new(false,true);
    readonly Form controls;
    readonly Label label = new() { AutoSize = true, Dock = DockStyle.Fill, TextAlign = ContentAlignment.MiddleLeft };
    readonly System.Windows.Forms.Timer timeout = new() { Interval = 300000 };
    bool done;
    public Selection(Screen screen,string side,double a,double b,bool smoke)
    {
        bounds=screen.Bounds; edge=side; vertical=side is "left" or "right";start=a;end=b;
        controls=new Form { Text="调整切换范围",FormBorderStyle=FormBorderStyle.FixedToolWindow,TopMost=true,ShowInTaskbar=false,StartPosition=FormStartPosition.Manual,AutoScaleMode=AutoScaleMode.Dpi,ClientSize=new Size(420,130) };
        controls.Location=new Point(screen.WorkingArea.Left+(screen.WorkingArea.Width-controls.Width)/2,screen.WorkingArea.Top+(screen.WorkingArea.Height-controls.Height)/2);
        var panel=new TableLayoutPanel {Dock=DockStyle.Fill,Padding=new Padding(14),RowCount=3};
        panel.Controls.Add(label);
        panel.Controls.Add(new Label {Text="拖动两个端点。桌面正常可用；Esc 取消。",AutoSize=true});
        var buttons=new FlowLayoutPanel {Dock=DockStyle.Fill,FlowDirection=FlowDirection.RightToLeft,AutoSize=true};
        var apply=new Button {Text="确认范围",AutoSize=true};var cancel=new Button {Text="取消",AutoSize=true};
        apply.Click+=(_,_)=>Finish(true);cancel.Click+=(_,_)=>Finish(false);
        buttons.Controls.Add(apply);buttons.Controls.Add(cancel);panel.Controls.Add(buttons);controls.Controls.Add(panel);
        controls.CancelButton=cancel;controls.FormClosing+=(_,_)=>Finish(false);
        foreach (var pair in new[]{(first,true),(last,false)})
        {
            var (form,isFirst)=pair;double initial=0;
            form.BeginDrag+=()=>initial=isFirst?start:end;
            form.Dragged+=(delta,p)=>{double t=Geometry.Drag(initial,delta,p,bounds,vertical);if(isFirst)start=Math.Min(t,end-.001);else end=Math.Max(t,start+.001);Refresh();};
        }
        Refresh();line.Show();first.Show();last.Show();controls.Show();
        SystemEvents.DisplaySettingsChanged+=DisplayChanged;
        timeout.Interval=smoke?500:300000;timeout.Tick+=(_,_)=>Finish(false);timeout.Start();
    }
    Point At(double t) => edge switch {
        "left"=>new(bounds.Left+2,bounds.Top+(int)Math.Round(t*(bounds.Height-1))),
        "right"=>new(bounds.Right-3,bounds.Top+(int)Math.Round(t*(bounds.Height-1))),
        "top"=>new(bounds.Left+(int)Math.Round(t*(bounds.Width-1)),bounds.Top+2),
        _=>new(bounds.Left+(int)Math.Round(t*(bounds.Width-1)),bounds.Bottom-3)
    };
    void Refresh()
    {
        var a=At(start);var b=At(end);
        line.Bounds=vertical?new Rectangle(a.X-2,Math.Min(a.Y,b.Y),4,Math.Max(1,Math.Abs(a.Y-b.Y))):new Rectangle(Math.Min(a.X,b.X),a.Y-2,Math.Max(1,Math.Abs(a.X-b.X)),4);
        foreach(var (form,p) in new[]{(first,a),(last,b)})form.Bounds=new Rectangle(Math.Clamp(p.X-12,bounds.Left,bounds.Right-24),Math.Clamp(p.Y-12,bounds.Top,bounds.Bottom-24),24,24);
        label.Text=$"起点 {start:P1} → 终点 {end:P1} · {edge}";
    }
    void DisplayChanged(object? sender,EventArgs e) { if(!done && !controls.IsDisposed) controls.BeginInvoke(new Action(()=>Finish(false))); }
    void Finish(bool apply)
    {
        if(done)return;done=true;timeout.Stop();SystemEvents.DisplaySettingsChanged-=DisplayChanged;
        Console.WriteLine(apply?JsonSerializer.Serialize(new {start,end}):"{\"cancelled\":true}");Console.Out.Flush();
        line.Dispose();first.Dispose();last.Dispose();controls.Dispose();timeout.Dispose();ExitThread();
    }
}

static class Program
{
    [STAThread] static int Main(string[] args)
    {
        try
        {
            if(args.SequenceEqual(new[]{"--self-test"})) {Geometry.Test();return 0;}
            Application.SetHighDpiMode(HighDpiMode.PerMonitorV2);Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);
            bool smoke=args.SequenceEqual(new[]{"--smoke-test"});
            if(smoke) {using var ui=new Selection(Screen.PrimaryScreen ?? throw new Exception("No screen"),"left",.2,.8,true);Application.Run(ui);return 0;}
            if(args.Length!=8)throw new ArgumentException("Expected x y width height edge start end scale");
            double Number(int n)=>double.Parse(args[n],CultureInfo.InvariantCulture);
            var rect=new[]{Number(0),Number(1),Number(2),Number(3)};double a=Number(5),b=Number(6),scale=Number(7);
            if(!double.IsFinite(a)||!double.IsFinite(b)||a<0||b>1||b-a<.000999999||!new[]{"left","right","top","bottom"}.Contains(args[4]))throw new ArgumentException("Invalid range");
            using var mutex=new Mutex(true,"Local\\LanMouseEdgeSelector",out bool created);
            if(!created)throw new Exception("An edge selector is already open");
            var screens=Screen.AllScreens.Where(s=>Geometry.Matches(s.Bounds,rect,scale)).ToArray();
            if(screens.Length!=1)throw new Exception("Monitor layout/DPI is ambiguous or changed; reopen range settings");
            if(screens[0].Bounds.Width<24||screens[0].Bounds.Height<24)throw new Exception("Monitor too small");
            using var selection=new Selection(screens[0],args[4],a,b,false);Application.Run(selection);return 0;
        }
        catch(Exception e) {Console.Error.WriteLine(e.Message);return 2;}
    }
}
