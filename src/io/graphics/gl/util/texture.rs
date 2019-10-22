use gl::{self, types::*};

pub struct Texture {
    tex: GLuint,
    x: i32,
    y: i32,
}

impl Texture {
    pub fn new(x: i32, y: i32) -> Texture {
        let mut tex: GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut tex);
        }

        let texture = Texture { tex, x, y };

        texture.bind();
        unsafe {
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
        }
        texture.unbind();

        texture
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.tex);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    pub fn upload_pixels<T>(&self, data: &[T]) {
        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGB as _,
                self.x,
                self.y,
                0,
                gl::RGB as _,
                gl::UNSIGNED_BYTE as _, // TODO: this is probably wrong
                data.as_ptr() as *const GLvoid,
            );
        }
    }
}
