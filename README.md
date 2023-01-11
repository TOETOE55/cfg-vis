# cfg-vis

A macro to support `#[cfg()]` on visibility.

```rust
use cfg_vis::cfg_vis;

// default visibility is `pub`, while the target is linux, the visibility is `pub(super)`.
#[cfg_vis(target_os = "linux", pub(super))]
pub fn foo() {}

```