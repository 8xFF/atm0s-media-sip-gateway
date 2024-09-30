use std::{future::Future, marker::PhantomData};

pub mod select2;
pub mod select3;

#[derive(Default)]
pub struct DummyFuture<O> {
    _tmp: PhantomData<O>,
}

impl<O> Future for DummyFuture<O> {
    type Output = O;

    fn poll(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        std::task::Poll::Pending
    }
}

impl<T1, T2, T3> From<select2::OrOutput<T1, T2>> for select3::OrOutput<T1, T2, T3> {
    fn from(value: select2::OrOutput<T1, T2>) -> Self {
        match value {
            select2::OrOutput::Left(v) => select3::OrOutput::Left(v),
            select2::OrOutput::Right(v) => select3::OrOutput::Middle(v),
        }
    }
}
