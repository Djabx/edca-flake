use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_owned(),
        Style::default().fg(crate::theme::accent()).add_modifier(Modifier::BOLD),
    ))
}

fn subheading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
}

fn ctrl(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {:<20}", key),
            Style::default().fg(crate::theme::accent()),
        ),
        Span::styled(desc.to_owned(), Style::default().fg(Color::White)),
    ])
}

fn note(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {text}"),
        Style::default().fg(Color::DarkGray),
    ))
}

fn blank() -> Line<'static> {
    Line::from("")
}

include!("logo_lines.rs");

pub(crate) fn build_lines() -> Vec<Line<'static>> {
    let mut lines = logo_lines();
    lines.push(blank());
    lines.extend(vec![
        Line::from(Span::styled(
            "EDCAS — Elite Dangerous Commander Assistant System",
            Style::default().fg(crate::theme::accent()).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Build {}", crate::build_info::BUILD),
            Style::default().fg(Color::DarkGray),
        )),
        blank(),
        Line::from(Span::styled(
            "A desktop assistant for Elite Dangerous commanders.",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Repository: https://github.com/arne-fuchs/edcas-client",
            Style::default().fg(Color::Cyan),
        )),
        blank(),

        // ── Global ───────────────────────────────────────────────
        heading("Global"),
        ctrl("1 … 9, 0", "Jump straight to a tab by its number"),
        ctrl("q / e",              "Previous / next tab"),
        ctrl("?",                  "Show/hide this help"),
        ctrl("Esc",                "Dismiss on-screen alert toasts"),
        note("Tab numbers are shown in the tab bar: 1 News · 2 Explorer · 3 Galaxy Map · 4 Stations"),
        note("5 Factions · 6 Trade · 7 Commander · 8 Carrier · 9 Workshop · 0 Todo · Settings via q/e"),
        blank(),

        // ── Navigation (shared by most views) ────────────────────
        heading("Navigation (most views)"),
        ctrl("w / ↑",             "Move cursor up"),
        ctrl("s / ↓",             "Move cursor down"),
        ctrl("d / →",             "Focus detail panel / next detail tab"),
        ctrl("a / ←",             "Focus list panel / previous detail tab"),
        blank(),

        // ── Stations & Carriers ───────────────────────────────────
        heading("Stations & Carriers"),
        ctrl("/ or f",             "Start a new search"),
        ctrl("Enter (typing)",     "Execute search"),
        ctrl("Esc (typing)",       "Cancel search"),
        ctrl("p",                  "Pin / unpin selected item"),
        ctrl("w / s",             "Move selection up / down in list"),
        ctrl("d / →",             "Open detail panel"),
        ctrl("a / ←",             "Back to list  /  prev detail tab"),
        subheading("Detail tabs: Overview · Market · Outfitting · Shipyard"),
        ctrl("d / →",             "Next detail tab"),
        ctrl("a / ←",             "Prev detail tab (or back to list)"),
        ctrl("w / s",             "Scroll detail content up / down"),
        blank(),

        // ── Carrier ──────────────────────────────────────────────
        heading("Carrier"),
        ctrl("a / d",             "Previous / next carrier"),
        ctrl("r",                  "Refresh the server's copy (market, outfitting, shipyard)"),
        note("Your own and squadron carriers are found in the journals once their management panel has been opened."),
        note("Mark other carriers as yours with m in Stations, or search for them in this tab."),
        blank(),

        // ── Factions ─────────────────────────────────────────────
        heading("Factions"),
        ctrl("/ or f",             "Start a new search"),
        ctrl("Enter (typing)",     "Execute search"),
        ctrl("Esc (typing)",       "Cancel search"),
        ctrl("p",                  "Pin / unpin selected faction"),
        ctrl("w / s",             "Move selection up / down in list"),
        ctrl("d / →",             "Open detail panel"),
        ctrl("a / ←",             "Back to list  /  prev detail tab"),
        subheading("Detail tabs: Info · Systems"),
        ctrl("d / →",             "Next detail tab"),
        ctrl("a / ←",             "Prev detail tab (or back to list)"),
        ctrl("w / s (Info)",      "Scroll info content up / down"),
        ctrl("w / s (Systems)",   "Select system row up / down"),
        ctrl("c (Systems)",       "Copy selected system name to clipboard"),
        blank(),

        // ── System ───────────────────────────────────────────────
        heading("System"),
        ctrl("w / s",             "Select faction in the faction list"),
        ctrl("Space",              "Open selected faction in the Factions tab"),
        blank(),

        // ── Explorer ─────────────────────────────────────────────
        heading("Explorer"),
        ctrl("w / s",             "Move up / down in the body tree"),
        ctrl("d / →",             "Expand node / open detail"),
        ctrl("a / ←",             "Collapse / go back"),
        blank(),

        // ── Galaxy Map ───────────────────────────────────────────
        heading("Galaxy Map"),
        ctrl("drag",               "Rotate the view"),
        ctrl("right-drag / shift", "Pan"),
        ctrl("wheel  or  + / -",   "Zoom"),
        ctrl("click / dbl-click",  "Select a star / fly to it"),
        ctrl("/",                  "Search a system by name (Enter takes the first match)"),
        ctrl("c / g",              "Center on your system / whole galaxy"),
        ctrl("W A S D",            "Move across the galaxy (hold; shift for faster)"),
        ctrl("↑ / ↓",              "Move in the selected system's body tree"),
        ctrl("Esc",                "Clear the selection"),
        note("Settings → Galaxy Map sets how many stars are loaded and drawn."),
        blank(),

        // ── Settings ─────────────────────────────────────────────
        heading("Settings"),
        ctrl("w / s  or  ↑/↓",    "Navigate rows"),
        ctrl("a  or  ←",          "Focus sidebar"),
        ctrl("d  or  →",          "Focus fields"),
        ctrl("Space",              "Select or begin editing a field"),
        ctrl("Enter",              "Save edited field"),
        ctrl("Esc",                "Cancel edit"),
        blank(),

        // ── Inventory ────────────────────────────────────────────
        heading("Inventory (Materials / Ship Locker)"),
        ctrl("d / a  or  →/←",   "Switch between Materials and Ship Locker tabs"),
        note("Raw, Manufactured, Encoded shown in three columns with colour-coded progress bars."),
        note("Ship Locker shows Items, Components, Consumables, Data in four columns."),
        blank(),

        // ── Closing the help ──────────────────────────────────────
        heading("This window"),
        ctrl("Esc / ? / q",       "Close this help"),
    ]);
    lines
}
