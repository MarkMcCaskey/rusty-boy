//! Code taken and inspired by http://nercury.github.io/rust/opengl/tutorial/
//! Thanks!

mod buffer;
pub mod data;
mod shader;
mod texture;

pub use self::buffer::{ArrayBuffer, ElementArrayBuffer, VertexArray};
pub use self::shader::{Program, Shader};
pub use texture::Texture;

use gl;
use std;
use std::ffi::{CStr, CString};
