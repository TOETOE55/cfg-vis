#![allow(dead_code)]

mod inner {
    use cfg_vis::cfg_vis;

    #[cfg_vis(test)]
    pub fn private_in_test() -> bool {
        false
    }

    #[cfg_vis(test, pub)]
    fn public_in_test() -> bool {
        true
    }

    #[cfg_vis(target_os = "windows", pub(super))]
    const PUBLIC_IN_WINDOWS: bool = true;

    #[cfg_vis(target_os = "macos", pub(crate))]
    pub(self) static PUBLIC_IN_MACOS: bool = true;
}

#[test]
fn it_works() {
    // assert!(inner::private_in_test()); can't compile,

    assert!(inner::public_in_test());

    #[cfg(target_os = "windows")]
    {
        assert!(inner::PUBLIC_IN_WINDOWS);
    }

    #[cfg(target_os = "macos")]
    {
        assert!(inner::PUBLIC_IN_WINDOWS);
    }
}