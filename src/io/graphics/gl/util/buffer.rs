use gl::{self, types::*};

pub struct ArrayBuffer {
    vbo: GLuint,
}

impl ArrayBuffer {
    pub fn new() -> ArrayBuffer {
        let mut vbo: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }

        ArrayBuffer { vbo }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }
    }

    pub fn static_draw_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.len() * ::std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }
}

impl Drop for ArrayBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
        }
    }
}

pub struct VertexArray {
    vao: GLuint,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut vao: GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
        }

        Self { vao }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }

    pub fn static_draw_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.len() * ::std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}

pub struct ElementArrayBuffer {
    ibo: GLuint,
}

impl ElementArrayBuffer {
    pub fn new() -> ElementArrayBuffer {
        let mut ibo: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut ibo);
        }

        ElementArrayBuffer { ibo }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ibo);
        }
    }

    pub fn unbind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
        }
    }

    pub fn static_draw_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (data.len() * ::std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }
}

impl Drop for ElementArrayBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.ibo);
        }
    }
}
