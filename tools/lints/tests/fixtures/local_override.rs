#[cfg_attr(dylint_lib = "idiomatic_lints", allow(nominal_boundary_type))]
pub mod interface {
    pub trait MeasureInterface {
        fn raw(&self) -> foreign::Raw;
    }
}
