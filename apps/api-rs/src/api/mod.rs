mod auth;
mod portal;
mod profile;
mod me;
mod admin;

pub use auth::router as auth_router;
pub use portal::router as portal_router;
pub use profile::router as profile_router;
pub use me::router as me_router;
pub use admin::router as admin_router;
