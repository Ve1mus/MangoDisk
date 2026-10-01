use mangodisk_core::system_resources::memory_analysis::{MemoryAnalysis, MemoryAnalysisService};

use super::error::{run_blocking, CommandResult};

#[tauri::command]
pub async fn analyze_memory() -> CommandResult<MemoryAnalysis> {
    run_blocking("analyze_memory", MemoryAnalysisService::analyze).await
}
