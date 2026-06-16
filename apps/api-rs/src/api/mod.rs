mod auth;
mod portal;
mod profile;
mod me;
mod admin;
mod setup;

pub use auth::router as auth_router;
pub use portal::router as portal_router;
pub use profile::router as profile_router;
pub use me::router as me_router;
pub use admin::router as admin_router;
pub use setup::router as setup_router;
