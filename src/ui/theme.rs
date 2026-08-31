use crate::category::ProcessCategory;
use windows_reactor::{Border, Brush, Color, ContentControl, CornerRadius, Thickness, View};

pub fn wrap_canvas(canvas: View) -> View {
    Border::new()
        .background(Brush::Solid(Color::argb(255, 28, 28, 30)))
        .border_brush(Brush::Solid(Color::argb(255, 60, 60, 65)))
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(CornerRadius::uniform(4.0))
        .content(canvas)
}

pub fn color_for_category(cat: ProcessCategory, seed: u32) -> Color {
    let base_hue = cat.base_hue();
    let offset = ((seed % 7) as f64) * 8.0 - 24.0;
    let hue = (base_hue + offset + 360.0) % 360.0;
    let (r, g, b) = hsl_to_rgb(hue, 0.60, 0.42);
    Color::argb(255, r, g, b)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r_prime, g_prime, b_prime) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r_prime + m) * 255.0).round() as u8,
        ((g_prime + m) * 255.0).round() as u8,
        ((b_prime + m) * 255.0).round() as u8,
    )
}
