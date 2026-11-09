//! Native local edge selection editor. Does not connect to or mutate the daemon.
use adw::prelude::*;
use gtk::{gdk, gio};
#[cfg(not(any(target_os = "macos", windows)))]
use input_event::edge_portal::drag_fraction;
use input_event::edge_portal::{Edge, PortalDraft};
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
                let _ = m;
                self.result.set_text(&if d.start == 0.0 && d.end == 1.0 {
                    "整个边缘".to_owned()
                } else {
                    format!(
                        "{:.1}% – {:.1}% · 覆盖 {:.1}%",
                        d.start * 100.0,
                        d.end * 100.0,
                        (d.end - d.start) * 100.0
                    )
                });
            }
            Err(e) => self.result.set_text(e),
        }
    }
}
#[cfg(any(not(any(target_os = "macos", windows)), test))]
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
    show_for_peer(app, None, "范围配置预览", "preview", 0);
}

pub fn show_for_peer(
    app: &adw::Application,
    parent: Option<&gtk::Window>,
    peer: &str,
    key: &str,
    position: u32,
) {
    let window_id = format!(
        "edge-range-{}",
        draft_path(key).file_stem().unwrap().to_string_lossy()
    );
    if let Some(existing) = app.windows().iter().find(|w| w.widget_name() == window_id) {
        existing.present();
        return;
    }
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
        .title("切换范围")
        .default_width(560)
        .default_height(480)
        .build();
    window.set_widget_name(&window_id);
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new("切换范围", peer)));
    window.set_titlebar(Some(&header));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    for set in [
        gtk::Widget::set_margin_top,
        gtk::Widget::set_margin_bottom,
        gtk::Widget::set_margin_start,
        gtk::Widget::set_margin_end,
    ] {
        set(body.upcast_ref(), 20);
    }
    let group = adw::PreferencesGroup::new();
    group.set_description(Some("选择本机屏幕，再到屏幕边缘拖动两个端点。"));
    body.append(&group);
    let monitor = gtk::ComboBoxText::new();
    for m in &monitors {
        monitor.append_text(m.label.split(" · ").next().unwrap_or(&m.label));
    }
    if !monitors.is_empty() {
        monitor.set_active(Some(0));
    }
    let monitor_row = adw::ActionRow::builder().title("屏幕").build();
    monitor.set_valign(gtk::Align::Center);
    monitor_row.add_suffix(&monitor);
    monitor_row.set_activatable_widget(Some(&monitor));
    group.add(&monitor_row);
    let edge = gtk::ComboBoxText::new();
    for s in ["左侧", "右侧", "顶部", "底部"] {
        edge.append_text(s);
    }
    edge.set_active(Some(position.min(3)));
    let id = gtk::Entry::builder()
        .text("default")
        .placeholder_text("与对端一致的通道名称")
        .build();
    let edge_row = adw::ActionRow::builder().title("边缘").build();
    edge.set_valign(gtk::Align::Center);
    edge_row.add_suffix(&edge);
    edge_row.set_activatable_widget(Some(&edge));
    group.add(&edge_row);
    let start = gtk::SpinButton::with_range(0.0, 100.0, 0.1);
    let end = gtk::SpinButton::with_range(0.0, 100.0, 0.1);
    start.set_digits(1);
    end.set_digits(1);
    end.set_value(100.0);
    let reversed = gtk::CheckButton::with_label("反转两端对应方向");
    let range = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    range.append(&gtk::Label::new(Some("起点 %")));
    range.append(&start);
    range.append(&gtk::Label::new(Some("终点 %")));
    range.append(&end);

    let result = gtk::Label::new(None);
    result.set_wrap(false);
    result.set_width_chars(20);
    result.set_xalign(1.0);
    result.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let selection_row = adw::ActionRow::builder().title("选定范围").build();
    selection_row.add_suffix(&result);
    group.add(&selection_row);
    let c = Controls {
        monitor,
        edge,
        start,
        end,
        reversed,
        id,
        result,
        monitors: Rc::new(monitors),
    };
    let select = gtk::Button::with_label("在屏幕上调整…");
    select.add_css_class("suggested-action");
    let local = c.clone();
    let parent = window.downgrade();
    select.connect_clicked(move |_| {
        if let Some(parent) = parent.upgrade() {
            select_on_screen(&local, &parent);
        }
    });
    body.append(&select);
    let advanced = adw::ExpanderRow::builder()
        .title("高级选项")
        .subtitle("精确范围、方向与配置文件")
        .build();
    let advanced_list = gtk::ListBox::new();
    advanced_list.add_css_class("boxed-list");
    advanced_list.set_selection_mode(gtk::SelectionMode::None);
    advanced_list.append(&advanced);
    body.append(&advanced_list);
    let range_row = adw::ActionRow::builder().title("范围百分比").build();
    range_row.add_suffix(&range);
    advanced.add_row(&range_row);
    let reverse_row = adw::ActionRow::builder().title("对应方向").build();
    reverse_row.add_suffix(&c.reversed);
    advanced.add_row(&reverse_row);
    let id_row = adw::ActionRow::builder().title("配对通道").build();
    id_row.add_suffix(&c.id);
    advanced.add_row(&id_row);
    let presets = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (name, a, b) in [
        ("整个边缘", 0.0, 100.0),
        ("前半段", 0.0, 50.0),
        ("中间一半", 25.0, 75.0),
        ("后半段", 50.0, 100.0),
    ] {
        let button = gtk::Button::with_label(name);
        let c = c.clone();
        button.connect_clicked(move |_| {
            c.start.set_value(a);
            c.end.set_value(b);
        });
        presets.append(&button);
    }
    let preset_row = adw::ActionRow::new();
    preset_row.add_suffix(&presets);
    advanced.add_row(&preset_row);
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
    for (label, save) in [("导入…", false), ("导出…", true)] {
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
    let file_row = adw::ActionRow::builder().title("配置文件").build();
    file_row.add_suffix(&files);
    advanced.add_row(&file_row);
    let note = gtk::Label::new(Some("实验功能 · 当前仅保存范围，尚未用于实际跨屏切换。"));
    note.add_css_class("dim-label");
    note.set_wrap(true);
    body.append(&note);
    let save = gtk::Button::with_label("保存范围");
    save.add_css_class("suggested-action");
    let target = draft_path(key);
    let saved = c.clone();
    let weak = window.downgrade();
    save.connect_clicked(move |_| {
        let result = (|| -> Result<(), String> {
            let d = saved.draft().map_err(str::to_owned)?;
            if !monitor_is_present(&d) {
                return Err("屏幕布局已变化，请重新打开范围设置".into());
            }
            let parent = target.parent().ok_or("Invalid config path")?;
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec_pretty(&d).map_err(|e| e.to_string())?;
            gio::File::for_path(&target)
                .replace_contents(
                    &bytes,
                    None,
                    true,
                    gio::FileCreateFlags::PRIVATE,
                    gio::Cancellable::NONE,
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                if let Some(w) = weak.upgrade() {
                    w.close();
                }
            }
            Err(e) => saved.result.set_text(&e),
        }
    });
    header.pack_end(&save);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&body)
        .build();
    root.append(&scroll);
    window.set_child(Some(&root));
    let mut restored = false;
    if let Ok(bytes) = std::fs::read(draft_path(key)) {
        if let Ok(d) = serde_json::from_slice::<PortalDraft>(&bytes) {
            if d.validate().is_ok() {
                let matches: Vec<_> = c
                    .monitors
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.rect == d.monitor_rect && m.label == d.monitor_label)
                    .collect();
                if matches.len() == 1 {
                    c.monitor.set_active(Some(matches[0].0 as u32));
                    c.edge.set_active(Some(match d.edge {
                        Edge::Left => 0,
                        Edge::Right => 1,
                        Edge::Top => 2,
                        Edge::Bottom => 3,
                    }));
                    c.id.set_text(&d.portal_id);
                    c.start.set_value(d.start * 100.0);
                    c.end.set_value(d.end * 100.0);
                    c.reversed.set_active(d.reversed);
                    restored = true;
                }
            }
        }
    }
    c.refresh();
    if draft_path(key).exists() && !restored {
        note.set_text("原范围无法匹配当前屏幕，请重新选择。保存前会保留原文件备份。");
    }
    window.connect_close_request(move |_| {
        for (object, id) in handlers.borrow_mut().drain(..) {
            object.disconnect(id);
        }
        gtk::glib::Propagation::Proceed
    });
    window.present();
}

/// A dedicated edit-only fullscreen surface on the selected physical monitor.
/// All handle positions are monitor-relative, not scaled preview coordinates.
#[cfg(not(any(target_os = "macos", windows)))]
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

#[cfg(any(target_os = "macos", windows))]
fn select_on_screen(c: &Controls, parent: &gtk::ApplicationWindow) {
    let d = match c.draft() {
        Ok(d) => d,
        Err(e) => {
            c.result.set_text(e);
            return;
        }
    };
    #[cfg(target_os = "macos")]
    let bundled = std::env::current_exe().ok().and_then(|p| {
        p.parent()
            .map(|p| p.join("../Helpers/EdgeOverlay.app/Contents/MacOS/edge-overlay"))
    });
    #[cfg(target_os = "macos")]
    let helper = bundled
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from(env!("LAN_MOUSE_EDGE_OVERLAY")));
    #[cfg(windows)]
    let helper = match std::env::current_exe().ok().and_then(|p| {
        p.parent()
            .map(|p| p.join("native-edge-overlay/edge-overlay.exe"))
    }) {
        Some(path) if path.is_file() => path,
        _ => {
            c.result
                .set_text("缺少 Windows 边缘选择组件，请使用完整发行包。");
            return;
        }
    };
    #[allow(unused_mut)]
    let mut args = vec![
        d.monitor_rect[0].to_string(),
        d.monitor_rect[1].to_string(),
        d.monitor_rect[2].to_string(),
        d.monitor_rect[3].to_string(),
        match d.edge {
            Edge::Left => "left",
            Edge::Right => "right",
            Edge::Top => "top",
            Edge::Bottom => "bottom",
        }
        .to_owned(),
        d.start.to_string(),
        d.end.to_string(),
    ];
    #[cfg(windows)]
    {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let list = display.monitors();
        let selected: Vec<_> = (0..list.n_items())
            .filter_map(|i| list.item(i)?.downcast::<gdk::Monitor>().ok())
            .filter(|m| {
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
                label == d.monitor_label && [r.x(), r.y(), r.width(), r.height()] == d.monitor_rect
            })
            .collect();
        if selected.len() != 1 {
            c.result.set_text("屏幕布局已变化，请重新打开设置。");
            return;
        }
        let m = &selected[0];
        let scale = if m.find_property("scale").is_some() {
            m.property::<f64>("scale")
        } else {
            f64::from(m.scale_factor())
        };
        args.push(scale.to_string());
    }
    let c = c.clone();
    let weak = parent.downgrade();
    parent.set_visible(false);
    gtk::glib::MainContext::default().spawn_local(async move {
        let output = gio::spawn_blocking(move || {
            let mut command = std::process::Command::new(helper);
            command.args(args);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            command.output()
        })
        .await;
        if let Some(parent) = weak.upgrade() {
            parent.present();
        }
        match output {
            Ok(Ok(o)) if o.status.success() => {
                let value = serde_json::from_slice::<serde_json::Value>(&o.stdout);
                match value {
                    Ok(v) if v.get("cancelled").and_then(|v| v.as_bool()) == Some(true) => {}
                    Ok(v) => {
                        if let (Some(a), Some(b)) = (v["start"].as_f64(), v["end"].as_f64()) {
                            let mut checked = d;
                            checked.start = a;
                            checked.end = b;
                            if checked.validate().is_ok() {
                                c.start.set_value(a * 100.0);
                                c.end.set_value(b * 100.0);
                                c.refresh();
                            } else {
                                c.result.set_text("Invalid overlay range");
                            }
                        } else {
                            c.result.set_text("Invalid overlay response");
                        }
                    }
                    Err(_) => c.result.set_text("Invalid overlay response"),
                }
            }
            Ok(Ok(o)) => c.result.set_text(&String::from_utf8_lossy(&o.stderr)),
            _ => c.result.set_text("Could not start the native edge overlay"),
        }
    });
}

fn draft_path(key: &str) -> std::path::PathBuf {
    // Fixed-length digest: arbitrary host names cannot escape or overflow filenames.
    use sha2::Digest;
    let name = format!("{:x}", sha2::Sha256::digest(key.as_bytes()));
    gtk::glib::user_config_dir()
        .join("lan-mouse")
        .join("edge-drafts")
        .join(format!("{name}.json"))
}

fn monitor_is_present(d: &PortalDraft) -> bool {
    let Some(display) = gdk::Display::default() else {
        return false;
    };
    let list = display.monitors();
    (0..list.n_items())
        .filter_map(|i| list.item(i)?.downcast::<gdk::Monitor>().ok())
        .filter(|m| {
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
            label == d.monitor_label && [r.x(), r.y(), r.width(), r.height()] == d.monitor_rect
        })
        .count()
        == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_storage_names_are_bounded_and_isolated() {
        let a = draft_path("../../unsafe/host:4242");
        let b = draft_path("other:4242");
        assert_ne!(a, b);
        assert_eq!(a.file_name().unwrap().to_str().unwrap().len(), 69);
        assert_eq!(a.parent(), b.parent());
        assert_eq!(
            draft_path(&"x".repeat(10000))
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .len(),
            69
        );
    }
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
        let r = (0.0, 0.0, 1919.0, 1079.0);
        assert_eq!(point(Edge::Right, 0.0, r), (r.0 + r.2, r.1));
        assert_eq!(point(Edge::Bottom, 1.0, r), (r.0 + r.2, r.1 + r.3));
    }
}
