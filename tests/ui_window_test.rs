use taskbar_clone::appbar::{calculate_appbar_rect, get_primary_monitor_rect};
use taskbar_clone::config::DockEdge;

slint::include_modules!();

#[test]
fn test_primary_monitor_window_sizing() {
    let app = MainWindow::new().expect("Failed to create MainWindow");
    let mon_rect = get_primary_monitor_rect();
    let bar_rect = calculate_appbar_rect(DockEdge::Bottom, 48, mon_rect);

    let bar_w = bar_rect.width() as u32;
    let bar_h = bar_rect.height() as u32;

    app.set_bar_width(bar_w as f32);
    app.set_bar_height(bar_h as f32);
    app.window()
        .set_size(slint::PhysicalSize::new(bar_w, bar_h));
    app.window()
        .set_position(slint::PhysicalPosition::new(bar_rect.left, bar_rect.top));

    let size = app.window().size();
    let pos = app.window().position();

    assert_eq!(size.width, bar_w);
    assert_eq!(size.height, bar_h);
    assert_eq!(pos.x, bar_rect.left);
    assert_eq!(pos.y, bar_rect.top);
}
