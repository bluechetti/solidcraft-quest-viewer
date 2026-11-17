//! Setting the model down on a real surface and keeping it there. The Quest's room-scan extensions are not
//! offered to apps by this headset's runtime, so the surface is found by touching it with the controller, and
//! the placement is kept in the STAGE space (fixed to the room's floor origin), so it is still there next time.

use openxr as xr;

/// The model's pose in the stage space (16 floats), kept in a file in the app's folder.
pub fn save_placement(path: &std::path::Path, rel: &[f32; 16]) {
    let nums: Vec<String> = rel.iter().map(|x| x.to_string()).collect();
    let _ = std::fs::write(path, format!("stage\n{}\n", nums.join(" ")));
}

pub fn load_placement(path: &std::path::Path) -> Option<[f32; 16]> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    if lines.next()?.trim() != "stage" {
        return None;
    }
    let nums: Vec<f32> = lines.next()?.split_whitespace().filter_map(|x| x.parse().ok()).collect();
    <[f32; 16]>::try_from(nums).ok()
}

pub fn forget_placement(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

/// Where the stage space is in `base`, if the headset knows it now.
pub fn stage_pose(stage: &xr::Space, base: &xr::Space, time: xr::Time) -> Option<xr::Posef> {
    let loc = stage.locate(base, time).ok()?;
    loc.location_flags.contains(xr::SpaceLocationFlags::POSITION_VALID | xr::SpaceLocationFlags::ORIENTATION_VALID).then_some(loc.pose)
}
