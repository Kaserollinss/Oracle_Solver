mod commands;
mod events;
mod state;
pub mod types;

use tauri_specta::{collect_commands, collect_events, Builder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::config::validate_board,
            commands::config::validate_config,
            commands::solve::start_solve,
            commands::solve::stop_solve,
            commands::solve::get_solve_status,
            commands::strategy::get_strategy_matrix,
            commands::strategy::get_hand_detail,
            commands::strategy::get_node_ev,
            commands::tree::get_tree_structure,
            commands::tree::get_node,
        ])
        .events(collect_events![
            events::SolveProgress,
            events::SolveComplete,
        ]);

    // In debug mode, export TypeScript bindings
    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default()
                .bigint(specta_typescript::BigIntExportBehavior::Number),
            "../src/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");

    let app_state = state::new_app_state();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .manage(app_state)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
