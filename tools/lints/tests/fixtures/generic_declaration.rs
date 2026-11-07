pub mod interface {
    pub trait ConvertInterface<Input> {
        fn convert<Replacement>(
            &self,
            input: Input,
            replacement: Replacement,
        ) -> (Input, Replacement);
    }
}
