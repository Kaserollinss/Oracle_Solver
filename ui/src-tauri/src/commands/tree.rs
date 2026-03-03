use crate::state::AppState;
use crate::types::TreeNodeDto;

#[tauri::command]
#[specta::specta]
pub fn get_tree_structure(state: tauri::State<'_, AppState>) -> Result<Vec<TreeNodeDto>, String> {
    log::debug!("get_tree_structure called");
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    let solver = session
        .solver
        .as_ref()
        .ok_or_else(|| "No active solve session".to_string())?;

    let tree = &solver.tree;
    let mut nodes = Vec::new();
    for node in &tree.nodes {
        // Only include decision nodes and chance nodes for the tree navigator
        if node.is_decision() || node.is_chance() {
            nodes.push(TreeNodeDto::from_node(node));
        }
    }
    log::debug!("Returning {} tree nodes", nodes.len());
    Ok(nodes)
}

#[tauri::command]
#[specta::specta]
pub fn get_node(node_id: u32, state: tauri::State<'_, AppState>) -> Result<TreeNodeDto, String> {
    log::debug!("get_node called: id={}", node_id);
    let session = state.lock().map_err(|e| format!("Lock error: {}", e))?;
    let solver = session
        .solver
        .as_ref()
        .ok_or_else(|| "No active solve session".to_string())?;

    let node = solver
        .tree
        .get(node_id)
        .ok_or_else(|| format!("Node {} not found", node_id))?;

    Ok(TreeNodeDto::from_node(node))
}
