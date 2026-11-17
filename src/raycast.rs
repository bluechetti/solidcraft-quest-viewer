//! XR_META_environment_raycast: where a ray from the controller meets the real room (table, floor, walls).
//! The extension is newer than the openxr-sys crate, so its structures and functions are declared here from
//! the OpenXR header (struct type values 1000592001..): create the raycaster (asynchronous, finished through an
//! XR_EXT_future future), then call `cast` each frame.

use openxr as xr;
use std::os::raw::{c_char, c_void};
use std::ptr;

#[repr(C)]
struct CreateInfo {
    ty: xr::sys::StructureType,
    next: *const c_void,
}

#[repr(C)]
struct CreateCompletion {
    ty: xr::sys::StructureType,
    next: *mut c_void,
    future_result: xr::sys::Result,
    raycaster: u64,
}

#[repr(C)]
struct HitGetInfo {
    ty: xr::sys::StructureType,
    next: *const c_void,
    base_space: xr::sys::Space,
    time: xr::Time,
    origin: xr::Vector3f,
    direction: xr::Vector3f,
    filter_count: u32,
    filters: *const *const c_void,
}

#[repr(C)]
struct Hit {
    ty: xr::sys::StructureType,
    next: *const c_void,
    status: i32,
    pose: xr::Posef,
}

type CreateAsync = unsafe extern "system" fn(xr::sys::Session, *const CreateInfo, *mut u64) -> xr::sys::Result;
type CreateComplete = unsafe extern "system" fn(xr::sys::Session, u64, *mut CreateCompletion) -> xr::sys::Result;
type Destroy = unsafe extern "system" fn(u64) -> xr::sys::Result;
type Perform = unsafe extern "system" fn(u64, *const HitGetInfo, *mut Hit) -> xr::sys::Result;

pub const EXTENSION_NAME: &str = "XR_META_environment_raycast";

/// What a cast found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cast {
    /// A hit: the point, and the direction the surface faces (a unit vector, best guess from the hit pose).
    Hit([f32; 3], [f32; 3]),
    /// Nothing usable (no hit, hidden behind something, or outside the cameras' view).
    Miss(i32),
}

pub struct EnvRaycast {
    create_async: CreateAsync,
    create_complete: CreateComplete,
    destroy: Destroy,
    perform: Perform,
    instance: xr::sys::Instance,
    session: xr::sys::Session,
    future: Option<u64>,
    handle: Option<u64>,
    /// Creation failed (no permission, or not supported): do not try again.
    pub failed: Option<String>,
}

fn ok(r: xr::sys::Result) -> bool {
    r == xr::sys::Result::SUCCESS
}

impl EnvRaycast {
    /// Loads the four functions; None when the headset does not have the extension.
    pub fn load(entry: &xr::Entry, instance: &xr::Instance, session: xr::sys::Session) -> Option<EnvRaycast> {
        let get = |name: &[u8]| -> Option<xr::sys::pfn::VoidFunction> {
            let mut f: Option<xr::sys::pfn::VoidFunction> = None;
            // SAFETY: a NUL-terminated name and a valid out-pointer.
            let r = unsafe { (entry.fp().get_instance_proc_addr)(instance.as_raw(), name.as_ptr() as *const c_char, &mut f) };
            if ok(r) { f } else { None }
        };
        // SAFETY: each pointer is the function the runtime named, with the signature from the OpenXR header.
        unsafe {
            Some(EnvRaycast {
                create_async: std::mem::transmute::<xr::sys::pfn::VoidFunction, CreateAsync>(get(b"xrCreateEnvironmentRaycasterAsyncMETA\0")?),
                create_complete: std::mem::transmute::<xr::sys::pfn::VoidFunction, CreateComplete>(get(b"xrCreateEnvironmentRaycasterCompleteMETA\0")?),
                destroy: std::mem::transmute::<xr::sys::pfn::VoidFunction, Destroy>(get(b"xrDestroyEnvironmentRaycasterMETA\0")?),
                perform: std::mem::transmute::<xr::sys::pfn::VoidFunction, Perform>(get(b"xrPerformEnvironmentRaycastMETA\0")?),
                instance: instance.as_raw(),
                session,
                future: None,
                handle: None,
                failed: None,
            })
        }
    }

    /// Moves creation along (call every frame until `ready`): starts it, then waits for its future.
    pub fn update(&mut self, instance: &xr::Instance) {
        if self.handle.is_some() || self.failed.is_some() {
            return;
        }
        if self.future.is_none() {
            let info = CreateInfo { ty: xr::sys::StructureType::from_raw(1000592001), next: ptr::null() };
            let mut f = 0u64;
            // SAFETY: valid session and structures.
            let r = unsafe { (self.create_async)(self.session, &info, &mut f) };
            if ok(r) {
                self.future = Some(f);
            } else {
                self.failed = Some(format!("{r:?}"));
            }
            return;
        }
        let Some(fut) = instance.exts().ext_future.as_ref() else {
            self.failed = Some("XR_EXT_future not loaded".into());
            return;
        };
        let future = self.future.unwrap();
        let info = xr::sys::FuturePollInfoEXT { ty: xr::sys::FuturePollInfoEXT::TYPE, next: ptr::null(), future };
        let mut res = xr::sys::FuturePollResultEXT { ty: xr::sys::FuturePollResultEXT::TYPE, next: ptr::null_mut(), state: xr::sys::FutureStateEXT::PENDING };
        // SAFETY: valid instance and structures.
        let r = unsafe { (fut.poll_future)(self.instance, &info, &mut res) };
        if !ok(r) {
            self.failed = Some(format!("poll {r:?}"));
        } else if res.state == xr::sys::FutureStateEXT::READY {
            let mut c = CreateCompletion { ty: xr::sys::StructureType::from_raw(1000592002), next: ptr::null_mut(), future_result: xr::sys::Result::SUCCESS, raycaster: 0 };
            // SAFETY: valid session, future and out-structure.
            let r = unsafe { (self.create_complete)(self.session, future, &mut c) };
            if ok(r) && ok(c.future_result) && c.raycaster != 0 {
                self.handle = Some(c.raycaster);
                log::info!("environment raycaster ready");
            } else {
                self.failed = Some(format!("complete {r:?} / {:?}", c.future_result));
            }
        }
    }

    /// Casts a ray (origin and unit direction in `base`) at the room.
    pub fn cast(&self, base: xr::sys::Space, time: xr::Time, origin: [f32; 3], dir: [f32; 3]) -> Option<Cast> {
        let handle = self.handle?;
        let info = HitGetInfo {
            ty: xr::sys::StructureType::from_raw(1000592003),
            next: ptr::null(),
            base_space: base,
            time,
            origin: xr::Vector3f { x: origin[0], y: origin[1], z: origin[2] },
            direction: xr::Vector3f { x: dir[0], y: dir[1], z: dir[2] },
            filter_count: 0,
            filters: ptr::null(),
        };
        let mut hit = Hit { ty: xr::sys::StructureType::from_raw(1000592004), next: ptr::null(), status: 0, pose: xr::Posef::IDENTITY };
        // SAFETY: valid handle and structures.
        let r = unsafe { (self.perform)(handle, &info, &mut hit) };
        if !ok(r) {
            return None;
        }
        if hit.status != 1 {
            return Some(Cast::Miss(hit.status));
        }
        let q = hit.pose.orientation;
        // The hit pose's orientation carries the surface normal; which axis is not documented, so take the
        // one of its y and z axes that is closer to vertical (a floor or table faces up), else z.
        let y = [2.0 * (q.x * q.y - q.w * q.z), 1.0 - 2.0 * (q.x * q.x + q.z * q.z), 2.0 * (q.y * q.z + q.w * q.x)];
        let z = [2.0 * (q.x * q.z + q.w * q.y), 2.0 * (q.y * q.z - q.w * q.x), 1.0 - 2.0 * (q.x * q.x + q.y * q.y)];
        let n = if y[1].abs() > z[1].abs() { y } else { z };
        Some(Cast::Hit([hit.pose.position.x, hit.pose.position.y, hit.pose.position.z], n))
    }
}

impl Drop for EnvRaycast {
    fn drop(&mut self) {
        if let Some(h) = self.handle.take() {
            // SAFETY: the handle was created by this runtime and is destroyed once.
            unsafe { (self.destroy)(h) };
        }
    }
}
