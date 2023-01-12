# cfg-vis

A macro to support `#[cfg()]` on visibility.

```rust
use cfg_vis::cfg_vis;
use cfg_vis::cfg_vis_fields;

// default visibility is `pub`, while the target is linux, the visibility is `pub(super)`.
#[cfg_vis(target_os = "linux", pub(super))]
pub fn foo() {}

// cfg_vis on fields
#[cfg_vis_fields]
struct Foo {
    // while the target is linux, the visibility is `pub`.
    #[cfg_vis(target_os = "linux", pub)]
    foo: i32,
}
```
