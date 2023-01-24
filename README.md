# cfg-vis

A macro to support `#[cfg()]` on visibility.

```rust
use cfg_vis::{cfg_vis, cfg_vis_fields};

// default visibility is `pub`, while the target is linux, the visibility is `pub(crate)`.
#[cfg_vis(target_os = "linux", pub(crate))]
pub fn foo() {}

#[cfg_vis_fields]
pub struct Foo {
    #[cfg_vis(test, pub)]
    pub_in_test: i32,
    #[cfg_vis(test)]
    pub prv_in_test: i32,
}
```
