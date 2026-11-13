mod gui;


use edcas_common::api::ConstructionDepotResponse;
use crate::todo::{ConstructionTodoItem, ConstructionTodoResource};

pub(super) use gui::{depot_ui, selection_marker, soften_table_highlights, DepotSummary, ResourceAction};

// ── Public helpers (used by stations.rs) ─────────────────────────────────────

pub(super) fn construction_todo_item_from_depot(
    depot: &crate::journal_reader::ConstructionDepotData,
) -> ConstructionTodoItem {
    let resources = depot.submission.resources.iter()
        .filter(|r| r.provided_amount < r.required_amount)
        .map(|r| ConstructionTodoResource {
            commodity_name: r.name.clone(),
            display_name: r.display_name.clone(),
            required_amount: r.required_amount,
            provided_amount: r.provided_amount,
            payment: r.payment,
        })
        .collect();
    ConstructionTodoItem {
        market_id: depot.submission.market_id,
        station_name: depot.submission.station_name.clone(),
        system_name: depot.system_name.clone(),
        resources,
    }
}

pub(super) fn construction_todo_item_from_response(depot: &ConstructionDepotResponse) -> ConstructionTodoItem {
    let resources = depot.resources.iter()
        .filter(|r| r.provided_amount < r.required_amount)
        .map(|r| ConstructionTodoResource {
            commodity_name: r.name.clone(),
            display_name: r.display_name.clone(),
            required_amount: r.required_amount,
            provided_amount: r.provided_amount,
            payment: r.payment,
        })
        .collect();
    ConstructionTodoItem {
        market_id: depot.market_id,
        station_name: depot.station_name.clone(),
        system_name: depot.system_name.clone(),
        resources,
    }
}

/// Returns the number of resource rows for a given depot (for navigation bounds).
pub(super) fn depot_resource_count_local(depot: &crate::journal_reader::ConstructionDepotData) -> usize {
    depot.submission.resources.len()
}

pub(super) fn depot_resource_count_api(depot: &ConstructionDepotResponse) -> usize {
    depot.resources.len()
}

// ── Private helpers ───────────────────────────────────────────────────────────

