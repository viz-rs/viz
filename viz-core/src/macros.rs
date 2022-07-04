macro_rules! tuple_impls {
    () => {
        tuple_impls!(@impl);
    };
    ($T:ident $( $U:ident )*) => {
        tuple_impls!($( $U )*);
        tuple_impls!(@impl $T $( $U )*);
    };
    // "Private" internal implementation
    (@impl $( $T:ident )*) => {
        #[async_trait]
        impl<$($T,)*> FromRequest for ($($T,)*)
        where
            $($T: FromRequest + Send + 'static,)*
            $($T::Error: IntoResponse + Send,)*
        {
            type Error = Error;

            #[allow(unused, unused_mut)]
            async fn extract(req: &mut Request<Body>) -> Result<($($T,)*), Self::Error> {
                Ok(($($T::extract(req).await.map_err(IntoResponse::into_error)?,)*))
            }
        }

        #[async_trait]
        impl<$($T,)* Fun, Fut, Out> FnExt<($($T,)*)> for Fun
        where
            $($T: FromRequest + Send + 'static,)*
            $($T::Error: IntoResponse + Send,)*
            Fun: Fn($($T,)*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Result<Out>> + Send,
            Out: IntoResponse + Send + Sync + 'static,
        {
            type Output =  Result<Response<Body>>;

            #[allow(unused, unused_mut)]
            async fn call(&self, mut req: Request<Body>) -> Self::Output {
                (self)($($T::extract(&mut req).await.map_err(IntoResponse::into_error)?,)*).await.map(IntoResponse::into_response)
            }
        }
    };
}
