//! Isolated compilation of the actual game Text converter, without linking Game.
#![allow(dead_code)]
trait TryIntoApi {
    type Api;
    type Error;
    fn try_into_api(&self) -> Result<Self::Api, Self::Error>;
}
trait TryIntoEngine<C: ?Sized> {
    type Engine;
    type Error;
    fn try_into_engine(&self, context: &C) -> Result<Self::Engine, Self::Error>;
}
#[path = "../../../../../crates/horizon-client/src/game/modding/api_values/text_boundary.rs"]
mod text_boundary;
