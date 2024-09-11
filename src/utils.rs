use std::path::{Path, Component};

pub fn has_traversal(input_path: &Path) -> bool {
    // Iterate over the components of the path
    for component in input_path.components() {
        // Check if any component is ParentDir, which means `..`
        if let Component::ParentDir = component {
            // Directory traversal attempt detected
            return true;
        }
    }
    // No traversal detected
    false
}
