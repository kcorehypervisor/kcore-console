//! kcore wordmark rendered as half-block cell art (generated from `assets/kcore-logo.png`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::theme;

mod data {
    include!(concat!(env!("OUT_DIR"), "/logo_data.rs"));
}

/// One pre-rendered size of the logo: rows of `(top, bottom)` half-cell classes.
pub type LogoRows = &'static [&'static [(u8, u8)]];

const BG: u8 = 0;
const VIOLET: u8 = 1;

/// Variants from largest to smallest.
const VARIANTS: [LogoRows; 3] = [data::LOGO_L, data::LOGO_M, data::LOGO_S];

/// Wordmark text used when no cell-art variant fits.
pub const WORDMARK: &str = "KCORE";

/// Pick the largest variant that fits inside `cols` x `rows`.
pub fn variant_for(cols: u16, rows: u16) -> Option<LogoRows> {
    VARIANTS.into_iter().find(|v| fits(v, cols, rows))
}

fn fits(v: &LogoRows, cols: u16, rows: u16) -> bool {
    width_of(v) <= cols && height_of(v) <= rows
}

/// Width in cells of a variant.
pub fn width_of(v: LogoRows) -> u16 {
    v.first().map(|r| r.len() as u16).unwrap_or(0)
}

/// Height in rows of a variant.
pub fn height_of(v: LogoRows) -> u16 {
    v.len() as u16
}

fn class_color(c: u8) -> Color {
    match c {
        BG => theme::bg(),
        VIOLET => theme::accent(),
        _ => theme::silver(),
    }
}

/// Renders the wordmark at the top-left of its area, falling back to text when it does not fit.
#[derive(Debug, Clone, Copy, Default)]
pub struct Logo;

impl Widget for Logo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        match variant_for(area.width, area.height) {
            Some(v) => render_cells(v, area, buf),
            None => render_wordmark(area, buf),
        }
    }
}

fn render_cells(v: LogoRows, area: Rect, buf: &mut Buffer) {
    for (dy, row) in v.iter().enumerate() {
        let y = area.y + dy as u16;
        for (dx, &(top, bottom)) in row.iter().enumerate() {
            let x = area.x + dx as u16;
            let (symbol, fg, bg) = match (top == BG, bottom == BG) {
                (true, true) => (" ", theme::bg(), theme::bg()),
                (false, false) if top == bottom => ("█", class_color(top), theme::bg()),
                // Two different inks in one cell: draw the top half on a bottom-coloured field.
                (false, false) => ("▀", class_color(top), class_color(bottom)),
                (false, true) => ("▀", class_color(top), theme::bg()),
                (true, false) => ("▄", class_color(bottom), theme::bg()),
            };
            buf[(x, y)]
                .set_symbol(symbol)
                .set_style(Style::default().fg(fg).bg(bg));
        }
    }
}

fn render_wordmark(area: Rect, buf: &mut Buffer) {
    let k = Style::default()
        .fg(theme::accent())
        .bg(theme::bg())
        .add_modifier(Modifier::BOLD);
    let rest = Style::default()
        .fg(theme::silver())
        .bg(theme::bg())
        .add_modifier(Modifier::BOLD);
    let mut x = area.x;
    for (i, ch) in WORDMARK.chars().enumerate() {
        if x >= area.right() {
            break;
        }
        let style = if i == 0 { k } else { rest };
        buf[(x, area.y)]
            .set_symbol(&ch.to_string())
            .set_style(style);
        x += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn variants_are_sized_as_generated() {
        assert_eq!((width_of(data::LOGO_L), height_of(data::LOGO_L)), (56, 6));
        assert_eq!((width_of(data::LOGO_M), height_of(data::LOGO_M)), (40, 4));
        assert_eq!((width_of(data::LOGO_S), height_of(data::LOGO_S)), (32, 3));
    }

    #[test]
    fn picks_largest_fitting_variant() {
        assert_eq!(variant_for(80, 10).map(height_of), Some(6));
        assert_eq!(variant_for(50, 6).map(height_of), Some(4));
        assert_eq!(variant_for(40, 3).map(height_of), Some(3));
        assert_eq!(variant_for(20, 3), None);
        assert_eq!(variant_for(80, 2), None);
    }

    #[test]
    fn renders_violet_k_and_white_core() {
        let area = Rect::new(0, 0, 56, 6);
        let mut buf = Buffer::empty(area);
        Logo.render(area, &mut buf);
        let mut violet = 0;
        let mut white = 0;
        for y in 0..6 {
            for x in 0..56 {
                let c = &buf[(x, y)];
                if c.symbol() == " " {
                    continue;
                }
                if c.fg == theme::accent() {
                    violet += 1;
                } else if c.fg == theme::silver() {
                    white += 1;
                }
            }
        }
        assert!(violet > 10, "violet K cells: {violet}");
        assert!(white > 40, "white core cells: {white}");
        // The K is on the left: the first inked column is violet.
        let first = (0..56)
            .find(|&x| (0..6).any(|y| buf[(x, y)].symbol() != " "))
            .expect("ink");
        let y = (0..6).find(|&y| buf[(first, y)].symbol() != " ").unwrap();
        assert_eq!(buf[(first, y)].fg, theme::accent());
    }

    #[test]
    fn falls_back_to_wordmark_when_short() {
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        Logo.render(area, &mut buf);
        let text: String = (0..5).map(|x| buf[(x, 0)].symbol().to_string()).collect();
        assert_eq!(text, WORDMARK);
        assert_eq!(buf[(0, 0)].fg, theme::accent());
        assert_eq!(buf[(1, 0)].fg, theme::silver());
    }
}
