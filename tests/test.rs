#![allow(dead_code)]

mod inner {
    use cfg_vis::{cfg_vis, cfg_vis_fields};

    #[cfg_vis(test)]
    pub fn private_in_test() -> bool {
        false
    }

    #[cfg_vis(test, pub)]
    fn public_in_test() -> bool {
        true
    }

    #[cfg_vis(target_os = "linux", pub)]
    #[cfg_vis(target_os = "windows", pub(super))]
    fn prv_in_macos() -> bool {
        true
    }

    #[cfg_vis(target_os = "windows", pub(super))]
    const PUBLIC_IN_WINDOWS: bool = true;

    #[cfg_vis(target_os = "macos", pub(crate))]
    pub(self) static PUBLIC_IN_MACOS: bool = true;

    #[cfg_vis_fields]
    pub struct Foo {
        #[cfg_vis(test, pub)]
        pub_in_test: i32,
        #[cfg_vis(test)]
        pub prv_in_test: i32,
    }

    #[cfg_vis_fields]
    pub struct Bar(#[cfg_vis(test, pub)] i32, #[cfg_vis(test)] pub i32);

    #[cfg_vis_fields]
    pub struct Baz {
        #[cfg_vis(target_os = "linux", pub)]
        #[cfg_vis(target_os = "windows", pub(super))]
        prv_in_macos: i32,
    }
}

// mod will_not_compile {
//     use cfg_vis::{cfg_vis, cfg_vis_fields};
//
//     fn call_private() {
//         crate::inner::private_in_test();
//     }
//
//     fn acc_struct_prv_fields(foo: crate::inner::Foo, bar: crate::inner::Bar) {
//         foo.prv_in_test;
//         bar.1;
//     }
//
//     #[cfg_vis(test, pub)]
//     #[cfg_vis(target_os = "windows", pub(super))]
//     fn dup_cfg() -> bool {
//         true
//     }
//
//     #[cfg_vis_fields]
//     struct DupAttr {
//         #[cfg_vis(test, pub)]
//         #[cfg_vis(target_os = "windows", pub(super))]
//         pub_in_test: i32,
//     }
//
//     #[cfg_vis(test, pub, dsfaodfads)]
//     fn wrong_arg1() -> bool {
//         true
//     }
//
//     #[cfg_vis]
//     fn wrong_arg2() -> bool {
//         true
//     }
//
//     #[cfg_vis_fields(sdfdsfa)]
//     struct WrongArgs;
// }

#[test]
fn it_works() {
    assert!(inner::public_in_test());

    #[cfg(target_os = "windows")]
    {
        assert!(inner::PUBLIC_IN_WINDOWS);
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        assert!(inner::prv_in_macos());
    }

    #[cfg(target_os = "macos")]
    {
        assert!(inner::PUBLIC_IN_WINDOWS);
    }
}

#[cfg(test)]
fn struct_fields_work(foo: inner::Foo, bar: inner::Bar, baz: inner::Baz) {
    foo.pub_in_test;
    bar.0;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        baz.prv_in_macos;
    }
}
