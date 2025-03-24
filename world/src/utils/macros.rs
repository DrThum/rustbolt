#[macro_export]
macro_rules! create_wrapped_resource {
    ($wrapper:ident, $inner:ty) => {
        #[derive(shipyard::Unique)]
        pub struct $wrapper(pub std::sync::Arc<$inner>);

        impl std::ops::Deref for $wrapper {
            type Target = std::sync::Arc<$inner>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $wrapper {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
