//! Native local edge selection editor. Does not connect to or mutate the daemon.
use gtk::{gdk, gio, prelude::*};
use input_event::edge_portal::{Edge, PortalDraft, drag_fraction};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Monitor {
    label: String,
    rect: [i32; 4],
}
#[derive(Clone)]
struct Controls {
    monitor: gtk::ComboBoxText,
    edge: gtk::ComboBoxText,
    start: gtk::SpinButton,
    end: gtk::SpinButton,
    reversed: gtk::CheckButton,
    id: gtk::Entry,
    result: gtk::Label,
    area: gtk::DrawingArea,
    monitors: Rc<Vec<Monitor>>,
}
impl Controls {
    fn draft(&self) -> Result<PortalDraft, &'static str> {
        let m = self
            .monitor
            .active()
            .and_then(|i| self.monitors.get(i as usize))
            .ok_or("No connected monitor selected")?;
        let d = PortalDraft {
            schema_version: 1,
            portal_id: self.id.text().to_string(),
            monitor_label: m.label.clone(),
            monitor_rect: m.rect,
            edge: match self.edge.active().unwrap_or(0) {
                0 => Edge::Left,
                1 => Edge::Right,
                2 => Edge::Top,
                _ => Edge::Bottom,
            },
            start: self.start.value() / 100.0,
            end: self.end.value() / 100.0,
            reversed: self.reversed.is_active(),
        };
        d.validate()?;
        Ok(d)
    }
    fn refresh(&self) {
        match self.draft() {
            Ok(d) => {
                let m = d.mapping().unwrap();
                self.result.set_text(&format!("Local coordinate {:.1} → {:.1}  •  midpoint {:.1}\nExport is a draft; live screen switching is not changed.",m.denormalize(0.0).unwrap(),m.denormalize(1.0).unwrap(),m.denormalize(0.5).unwrap()));
            }
            Err(e) => self.result.set_text(e),
        }
        self.area.queue_draw();
    }
}
fn frame(width: i32, height: i32, rect: [i32; 4]) -> (f64, f64, f64, f64) {
    let scale = ((f64::from(width) - 100.0) / f64::from(rect[2]))
        .min((f64::from(height) - 80.0) / f64::from(rect[3]))
        .max(0.001);
    let (w, h) = (f64::from(rect[2]) * scale, f64::from(rect[3]) * scale);
    (
        (f64::from(width) - w) / 2.0,
        (f64::from(height) - h) / 2.0,
        w,
        h,
    )
}
fn point(edge: Edge, t: f64, r: (f64, f64, f64, f64)) -> (f64, f64) {
    let (x, y, w, h) = r;
    match edge {
        Edge::Left => (x, y + h * t),
        Edge::Right => (x + w, y + h * t),
        Edge::Top => (x + w * t, y),
        Edge::Bottom => (x + w * t, y + h),
    }
}
pub fn show(app: &adw::Application) {
    let monitors: Vec<_> = gdk::Display::default()
        .map(|d| {
            let list = d.monitors();
            (0..list.n_items())
                .filter_map(|i| list.item(i)?.downcast::<gdk::Monitor>().ok())
                .map(|m| {
                    let r = m.geometry();
                    Monitor {
                        label: format!(
                            "{} {} · {}×{} @ {},{}",
                            m.manufacturer().unwrap_or_default(),
                            m.model().unwrap_or_default(),
                            r.width(),
                            r.height(),
                            r.x(),
                            r.y()
                        ),
                        rect: [r.x(), r.y(), r.width(), r.height()],
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Lan Mouse · Edge range editor")
        .default_width(850)
        .default_height(680)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for set in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        set(body.upcast_ref(), 20);
    }
    let title = gtk::Label::new(Some("Choose a local screen edge"));
    title.add_css_class("title-2");
    body.append(&title);
    let monitor = gtk::ComboBoxText::new();
    for m in &monitors {
        monitor.append_text(&m.label);
    }
    if !monitors.is_empty() {
        monitor.set_active(Some(0));
    }
    body.append(&monitor);
    let edge = gtk::ComboBoxText::new();
    for s in ["Left", "Right", "Top", "Bottom"] {
        edge.append_text(s);
    }
    edge.set_active(Some(0));
    let id = gtk::Entry::builder()
        .text("mac-windows")
        .placeholder_text("Matching portal ID on both computers")
        .build();
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.append(&gtk::Label::new(Some("Portal")));
    row.append(&id);
    row.append(&edge);
    body.append(&row);
    let area = gtk::DrawingArea::builder()
        .content_width(700)
        .content_height(330)
        .hexpand(true)
        .vexpand(true)
        .build();
    body.append(&area);
    let start = gtk::SpinButton::with_range(0.0, 100.0, 0.1);
    let end = gtk::SpinButton::with_range(0.0, 100.0, 0.1);
    start.set_digits(1);
    end.set_digits(1);
    end.set_value(100.0);
    let reversed = gtk::CheckButton::with_label("Reverse correspondence");
    let range = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    range.append(&gtk::Label::new(Some("Start %")));
    range.append(&start);
    range.append(&gtk::Label::new(Some("End %")));
    range.append(&end);
    range.append(&reversed);
    body.append(&range);
    let result = gtk::Label::new(None);
    result.set_wrap(true);
    body.append(&result);
    let c = Controls {
        monitor,
        edge,
        start,
        end,
        reversed,
        id,
        result,
        area,
        monitors: Rc::new(monitors),
    };
    let select = gtk::Button::with_label("在真实屏幕边缘选择范围…");
    select.add_css_class("suggested-action");
    let local = c.clone();
    let parent = window.downgrade();
    select.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            select_on_screen(&local, &parent);
        }
    });
    body.prepend(&select);
    let presets = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (name, a, b) in [
        ("Whole edge", 0.0, 100.0),
        ("First half", 0.0, 50.0),
        ("Centered half", 25.0, 75.0),
        ("Last half", 50.0, 100.0),
    ] {
        let button = gtk::Button::with_label(name);
        let c = c.clone();
        button.connect_clicked(move |_| {
            c.start.set_value(a);
            c.end.set_value(b);
        });
        presets.append(&button);
    }
    body.append(&presets);
    let draw = c.clone();
    c.area.set_draw_func(move |_, ctx, w, h| {
        ctx.set_source_rgb(0.08, 0.10, 0.14);
        let _ = ctx.paint();
        let Some(m) = draw
            .monitor
            .active()
            .and_then(|i| draw.monitors.get(i as usize))
        else {
            return;
        };
        let r = frame(w, h, m.rect);
        ctx.set_source_rgb(0.20, 0.25, 0.33);
        ctx.rectangle(r.0, r.1, r.2, r.3);
        let _ = ctx.fill();
        ctx.set_source_rgb(0.65, 0.72, 0.82);
        ctx.set_font_size(15.0);
        ctx.move_to(r.0 + 15.0, r.1 + 25.0);
        let _ = ctx.show_text("Drag the blue endpoints");
        if let Ok(d) = draw.draft() {
            let a = point(d.edge, d.start, r);
            let b = point(d.edge, d.end, r);
            ctx.set_source_rgb(0.25, 0.68, 1.0);
            ctx.set_line_width(6.0);
            ctx.move_to(a.0, a.1);
            ctx.line_to(b.0, b.1);
            let _ = ctx.stroke();
            for (i, p) in [a, b].iter().enumerate() {
                ctx.arc(p.0, p.1, 9.0, 0.0, std::f64::consts::TAU);
                let _ = ctx.fill();
                ctx.move_to(p.0 + 12.0, p.1);
                let label = if (i == 0) != d.reversed { "0" } else { "1" };
                let _ = ctx.show_text(label);
            }
        }
    });
    let drag = gtk::GestureDrag::new();
    let active = Rc::new(RefCell::new(None::<(bool, f64, f64)>));
    let begin = c.clone();
    let selected = active.clone();
    drag.connect_drag_begin(move |_, x, y| {
        *selected.borrow_mut() = None;
        let Ok(d) = begin.draft() else { return };
        let r = frame(begin.area.width(), begin.area.height(), d.monitor_rect);
        for (is_start, t) in [(true, d.start), (false, d.end)] {
            let p = point(d.edge, t, r);
            if (p.0 - x).hypot(p.1 - y) < 24.0 {
                *selected.borrow_mut() = Some((is_start, x, y));
                break;
            }
        }
    });
    let change = c.clone();
    let selected = active.clone();
    drag.connect_drag_update(move |_, dx, dy| {
        let Some((first, x, y)) = *selected.borrow() else {
            return;
        };
        let Ok(d) = change.draft() else { return };
        let r = frame(change.area.width(), change.area.height(), d.monitor_rect);
        let vertical = matches!(d.edge, Edge::Left | Edge::Right);
        let Some(t) = drag_fraction(
            if vertical { y + dy } else { x + dx },
            if vertical { r.1 } else { r.0 },
            if vertical { r.3 } else { r.2 },
        ) else {
            return;
        };
        if first {
            change
                .start
                .set_value((t * 100.0).min(change.end.value() - 0.1));
        } else {
            change
                .end
                .set_value((t * 100.0).max(change.start.value() + 0.1));
        }
    });
    drag.connect_drag_end(move |_, _, _| {
        *active.borrow_mut() = None;
    });
    let drag_cleanup = drag.clone();
    c.area.add_controller(drag);
    let handlers = Rc::new(RefCell::new(Vec::<(
        gtk::glib::Object,
        gtk::glib::SignalHandlerId,
    )>::new()));
    for spin in [&c.start, &c.end] {
        let c = c.clone();
        let id = spin.connect_value_changed(move |_| c.refresh());
        handlers.borrow_mut().push((spin.clone().upcast(), id));
    }
    for combo in [&c.monitor, &c.edge] {
        let c = c.clone();
        let id = combo.connect_changed(move |_| c.refresh());
        handlers.borrow_mut().push((combo.clone().upcast(), id));
    }
    let changed = c.clone();
    let handler = c.id.connect_changed(move |_| changed.refresh());
    handlers.borrow_mut().push((c.id.clone().upcast(), handler));
    let changed = c.clone();
    let handler = c.reversed.connect_toggled(move |_| changed.refresh());
    handlers
        .borrow_mut()
        .push((c.reversed.clone().upcast(), handler));
    let files = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, save) in [("Import draft…", false), ("Export draft…", true)] {
        let button = gtk::Button::with_label(label);
        let c = c.clone();
        let parent = window.downgrade();
        button.connect_clicked(move |_| {
            let Some(parent)=parent.upgrade()else{return};
            let snapshot=if save {match c.draft(){Ok(d)=>Some(d),Err(e)=>{c.result.set_text(e);return}}}else{None};
            let dialog=gtk::FileChooserNative::new(Some(if save{"Export local portal"}else{"Import local portal"}),Some(&parent),if save{gtk::FileChooserAction::Save}else{gtk::FileChooserAction::Open},Some(if save{"Export"}else{"Import"}),Some("Cancel"));
            if save{dialog.set_current_name("local-edge-portal.json");}
            let c=c.clone();dialog.connect_response(move |dialog,response|{
                if response!=gtk::ResponseType::Accept{return}let Some(path)=dialog.file().and_then(|f|f.path())else{c.result.set_text("Select a local file");return};
                let outcome:Result<(),String>=(||{
                    if let Some(d)=&snapshot {
                        let display=gdk::Display::default().ok_or("No display")?;
                        let current=display.monitors();
                        let present=(0..current.n_items()).filter_map(|i|current.item(i)?.downcast::<gdk::Monitor>().ok()).any(|m|{let r=m.geometry();let label=format!("{} {} · {}×{} @ {},{}",m.manufacturer().unwrap_or_default(),m.model().unwrap_or_default(),r.width(),r.height(),r.x(),r.y());label==d.monitor_label && [r.x(),r.y(),r.width(),r.height()]==d.monitor_rect});
                        if !present {return Err("Screen layout changed; reopen the editor before exporting".into())}
                        let data=serde_json::to_vec_pretty(d).map_err(|e|e.to_string())?;gio::File::for_path(&path).replace_contents(&data,None,false,gio::FileCreateFlags::NONE,gio::Cancellable::NONE).map_err(|e|e.to_string())?;}
                    else {let meta=std::fs::metadata(&path).map_err(|e|e.to_string())?;if meta.len()>65536{return Err("Draft exceeds 64 KiB".into())}let mut data=Vec::new();use std::io::Read;std::fs::File::open(&path).map_err(|e|e.to_string())?.take(65537).read_to_end(&mut data).map_err(|e|e.to_string())?;if data.len()>65536{return Err("Draft exceeds 64 KiB".into())}let d:PortalDraft=serde_json::from_slice(&data).map_err(|e|e.to_string())?;d.validate().map_err(str::to_owned)?;
                        let matches:Vec<_>=c.monitors.iter().enumerate().filter(|(_,m)|m.label==d.monitor_label&&m.rect==d.monitor_rect).collect();if matches.len()!=1{return Err("Monitor missing, changed, or ambiguous; reselect its range manually".into())}
                        c.monitor.set_active(Some(matches[0].0 as u32));c.edge.set_active(Some(match d.edge{Edge::Left=>0,Edge::Right=>1,Edge::Top=>2,Edge::Bottom=>3}));c.id.set_text(&d.portal_id);c.start.set_value(d.start*100.0);c.end.set_value(d.end*100.0);c.reversed.set_active(d.reversed);c.refresh();}
                    Ok(())})();c.result.set_text(&match outcome{Ok(())=>if save{"Draft exported. Live configuration unchanged.".into()}else{"Draft imported; edit and export to save changes.".into()},Err(e)=>e});
            });dialog.show();
        });
        files.append(&button);
    }
    body.append(&files);
    let note = gtk::Label::new(Some(
        "Each computer chooses its own interval. Use the same portal ID on its peer.\nClosing discards unsaved edits. Screen changes require reopening this editor.",
    ));
    note.set_wrap(true);
    body.append(&note);
    window.set_child(Some(&body));
    c.refresh();
    let cleanup = c.clone();
    window.connect_close_request(move |_| {
        cleanup.area.unset_draw_func();
        cleanup.area.remove_controller(&drag_cleanup);
        for (object, id) in handlers.borrow_mut().drain(..) {
            object.disconnect(id);
        }
        gtk::glib::Propagation::Proceed
    });
    window.present();
}

/// A dedicated edit-only fullscreen surface on the selected physical monitor.
/// All handle positions are monitor-relative, not scaled preview coordinates.
fn select_on_screen(c: &Controls, parent: &gtk::ApplicationWindow) {
    let draft = match c.draft() {
        Ok(d) => d,
        Err(e) => {
            c.result.set_text(e);
            return;
        }
    };
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let monitors = display.monitors();
    let selected = (0..monitors.n_items())
        .filter_map(|i| monitors.item(i)?.downcast::<gdk::Monitor>().ok())
        .find(|m| {
            let r = m.geometry();
            let label = format!(
                "{} {} · {}×{} @ {},{}",
                m.manufacturer().unwrap_or_default(),
                m.model().unwrap_or_default(),
                r.width(),
                r.height(),
                r.x(),
                r.y()
            );
            label == draft.monitor_label
                && [r.x(), r.y(), r.width(), r.height()] == draft.monitor_rect
        });
    let Some(monitor) = selected else {
        c.result
            .set_text("Screen layout changed. Reopen the editor.");
        return;
    };
    let window = gtk::ApplicationWindow::builder()
        .application(&parent.application().unwrap())
        .title("选择真实屏幕边缘范围 · Esc 取消")
        .decorated(false)
        .build();
    window.add_css_class("edge-overlay");
    let css = gtk::CssProvider::new();
    css.load_from_data("window.edge-overlay { background-color: rgba(12, 18, 28, 0.18); } .edge-controls { background-color: rgba(20, 28, 40, 0.96); color: white; border-radius: 16px; padding: 20px; }");
    gtk::style_context_add_provider_for_display(
        &display,
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let overlay = gtk::Overlay::new();
    let area = gtk::DrawingArea::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    overlay.set_child(Some(&area));
    let range = Rc::new(RefCell::new((draft.start, draft.end)));
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    panel.add_css_class("edge-controls");
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    let title = gtk::Label::new(Some("点击屏幕边缘放置端点，或拖动蓝色端点"));
    title.add_css_class("title-3");
    panel.append(&title);
    let name = gtk::Label::new(Some(&draft.monitor_label));
    panel.append(&name);
    let value = gtk::Label::new(None);
    panel.append(&value);
    panel.append(&gtk::Label::new(Some(
        "蓝线就是所选切换范围。确认前不会修改任何配置。",
    )));
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    buttons.set_halign(gtk::Align::Center);
    let cancel = gtk::Button::with_label("取消 · Esc");
    let apply = gtk::Button::with_label("确认范围");
    apply.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&apply);
    panel.append(&buttons);
    overlay.add_overlay(&panel);
    let edge = draft.edge;
    let update: Rc<dyn Fn()> = {
        let range = range.clone();
        let value = value.clone();
        let weak = area.downgrade();
        Rc::new(move || {
            let (a, b) = *range.borrow();
            value.set_text(&format!(
                "起点 {:.1}%   →   终点 {:.1}%",
                a * 100.0,
                b * 100.0
            ));
            if let Some(area) = weak.upgrade() {
                area.queue_draw();
            }
        })
    };
    let r = range.clone();
    area.set_draw_func(move |_, ctx, w, h| {
        let w = f64::from(w);
        let h = f64::from(h);
        let (a, b) = *r.borrow();
        let geometry = (0.0, 0.0, w - 1.0, h - 1.0);
        let p = point(edge, a, geometry);
        let q = point(edge, b, geometry);
        ctx.set_source_rgba(0.1, 0.65, 1.0, 0.3);
        ctx.set_line_width(18.0);
        let full_a = point(edge, 0.0, geometry);
        let full_b = point(edge, 1.0, geometry);
        ctx.move_to(full_a.0, full_a.1);
        ctx.line_to(full_b.0, full_b.1);
        let _ = ctx.stroke();
        ctx.set_source_rgb(0.1, 0.75, 1.0);
        ctx.set_line_width(8.0);
        ctx.move_to(p.0, p.1);
        ctx.line_to(q.0, q.1);
        let _ = ctx.stroke();
        for (i, (x, y)) in [p, q].into_iter().enumerate() {
            ctx.arc(x, y, 18.0, 0.0, std::f64::consts::TAU);
            let _ = ctx.fill();
            ctx.set_font_size(20.0);
            ctx.move_to(x.clamp(25.0, w - 150.0), y.clamp(25.0, h - 25.0));
            let _ = ctx.show_text(if i == 0 { "START" } else { "END" });
        }
    });
    let drag = gtk::GestureDrag::new();
    let active = Rc::new(RefCell::new(None::<(bool, f64, f64)>));
    let state = active.clone();
    let r = range.clone();
    let weak = area.downgrade();
    drag.connect_drag_begin(move |_, x, y| {
        *state.borrow_mut() = None;
        let Some(area) = weak.upgrade() else { return };
        let w = f64::from(area.width());
        let h = f64::from(area.height());
        let vertical = matches!(edge, Edge::Left | Edge::Right);
        let distance = match edge {
            Edge::Left => x,
            Edge::Right => w - x,
            Edge::Top => y,
            Edge::Bottom => h - y,
        };
        if distance > 48.0 {
            return;
        }
        let t = if vertical {
            y / (h - 1.0)
        } else {
            x / (w - 1.0)
        };
        let (a, b) = *r.borrow();
        *state.borrow_mut() = Some(((t - a).abs() <= (t - b).abs(), x, y));
    });
    let state = active.clone();
    let r = range.clone();
    let weak = area.downgrade();
    let changed = update.clone();
    drag.connect_drag_update(move |_, dx, dy| {
        let Some((first, x, y)) = *state.borrow() else {
            return;
        };
        let Some(area) = weak.upgrade() else { return };
        let vertical = matches!(edge, Edge::Left | Edge::Right);
        let span = f64::from(if vertical {
            area.height()
        } else {
            area.width()
        }) - 1.0;
        let Some(t) = drag_fraction(if vertical { y + dy } else { x + dx }, 0.0, span) else {
            return;
        };
        let mut pair = r.borrow_mut();
        if first {
            pair.0 = t.min(pair.1 - 0.001).max(0.0);
        } else {
            pair.1 = t.max(pair.0 + 0.001).min(1.0);
        }
        drop(pair);
        changed();
    });
    drag.connect_drag_end(move |_, _, _| {
        *active.borrow_mut() = None;
    });
    let click = gtk::GestureClick::new();
    let r = range.clone();
    let weak = area.downgrade();
    let changed = update.clone();
    click.connect_pressed(move |_, _, x, y| {
        let Some(area) = weak.upgrade() else { return };
        let w = f64::from(area.width());
        let h = f64::from(area.height());
        let distance = match edge {
            Edge::Left => x,
            Edge::Right => w - x,
            Edge::Top => y,
            Edge::Bottom => h - y,
        };
        if distance > 48.0 {
            return;
        }
        let vertical = matches!(edge, Edge::Left | Edge::Right);
        let Some(t) = drag_fraction(
            if vertical { y } else { x },
            0.0,
            if vertical { h - 1.0 } else { w - 1.0 },
        ) else {
            return;
        };
        let mut pair = r.borrow_mut();
        if (t - pair.0).abs() <= (t - pair.1).abs() {
            pair.0 = t.min(pair.1 - 0.001).max(0.0);
        } else {
            pair.1 = t.max(pair.0 + 0.001).min(1.0);
        }
        drop(pair);
        changed();
    });
    let click_group = click.clone();
    let drag_group = drag.clone();
    area.add_controller(click);
    area.add_controller(drag);
    click_group.group_with(&drag_group);
    let weak = window.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(w) = weak.upgrade() {
            w.close();
        }
    });
    let weak = window.downgrade();
    let c = c.clone();
    apply.connect_clicked(move |_| {
        let (a, b) = *range.borrow();
        c.start.set_value(a * 100.0);
        c.end.set_value(b * 100.0);
        c.refresh();
        if let Some(w) = weak.upgrade() {
            w.close();
        }
    });
    let keys = gtk::EventControllerKey::new();
    let weak = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(w) = weak.upgrade() {
                w.close();
            }
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);
    let weak = parent.downgrade();
    window.connect_close_request(move |_| {
        gtk::style_context_remove_provider_for_display(&display, &css);
        if let Some(parent) = weak.upgrade() {
            parent.present();
        }
        gtk::glib::Propagation::Proceed
    });
    window.set_child(Some(&overlay));
    update();
    window.fullscreen_on_monitor(&monitor);
    window.present();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn draft_round_trip_and_unknown_fields() {
        let d = PortalDraft {
            schema_version: 1,
            portal_id: "pair".into(),
            monitor_label: "monitor".into(),
            monitor_rect: [-1920, 0, 1920, 1080],
            edge: Edge::Right,
            start: 0.2,
            end: 0.8,
            reversed: true,
        };
        let json = serde_json::to_string(&d).unwrap();
        let parsed: PortalDraft = serde_json::from_str(&json).unwrap();
        assert!(parsed.validate().is_ok());
        assert_eq!(
            parsed.mapping().unwrap().denormalize(0.5),
            d.mapping().unwrap().denormalize(0.5)
        );
        assert!(
            serde_json::from_str::<PortalDraft>(&json.replacen('{', "{\"unexpected\":1,", 1))
                .is_err()
        );
    }
    #[test]
    fn preview_aspect_and_endpoints() {
        let r = frame(800, 600, [-200, 0, 1600, 900]);
        assert!((r.2 / r.3 - 1600.0 / 900.0).abs() < 1e-12);
        assert_eq!(point(Edge::Right, 0.0, r), (r.0 + r.2, r.1));
        assert_eq!(point(Edge::Bottom, 1.0, r), (r.0 + r.2, r.1 + r.3));
    }
}
