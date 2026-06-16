use std::path::PathBuf;
use hdl_graph_core::*;
use crate::server::ProjectState;

/// Export the code graph to a file (SCIP, JSON, or Markdown).
pub fn run(state: &ProjectState, format: &str, output: &str) -> String {
    let path = PathBuf::from(output);

    match format {
        "scip" => {
            match hdl_graph_query::ScipExporter::export(
                &state.graph, &state.symbols, &state.file_map, &path,
            ) {
                Ok(()) => format!("Exported SCIP index to {}", path.display()),
                Err(e) => format!("Export failed: {}", e),
            }
        }
        "json" => {
            match hdl_graph_query::JsonExporter::export(
                &state.graph, &state.symbols, &state.file_map, &path,
            ) {
                Ok(()) => format!("Exported JSON graph to {}", path.display()),
                Err(e) => format!("Export failed: {}", e),
            }
        }
        "markdown" => {
            match hdl_graph_query::MarkdownExporter::export(
                &state.graph, &state.symbols, &state.file_map, &path,
                hdl_graph_query::MarkdownMode::Single,
            ) {
                Ok(()) => format!("Exported Markdown to {}", path.display()),
                Err(e) => format!("Export failed: {}", e),
            }
        }
        _ => format!(
            "Unknown export format '{}'. Valid formats: scip, json, markdown",
            format
        ),
    }
}
