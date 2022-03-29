use std::any::Any;
use std::cell::RefCell;
use std::fs::FileType;
use std::io::{IoSlice, IoSliceMut, SeekFrom};
use tokio::io::AsyncRead;
use tokio::sync::Mutex;
use wasi_common::file::FdFlags;
use wasi_common::ErrorExt;
use wasmtime_wasi::{Error, WasiFile};

struct Inner<T> {
    buf: [u8; 4096],
    buf_len: usize,
    reader: T,
}

pub struct RequestBodyFile<T> {
    inner: Mutex<Inner<T>>,
}

impl<T> RequestBodyFile<T> {
    pub(crate) fn new(reader: T) -> Self {
        Self {
            inner: Mutex::new(Inner {
                buf: [0; 4096],
                buf_len: 0,
                reader,
            }),
        }
    }
}

#[poem::async_trait]
impl<T: AsyncRead + Unpin> WasiFile for RequestBodyFile<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn sock_accept(
        &mut self,
        _fdflags: wasi_common::file::FdFlags,
    ) -> Result<Box<dyn WasiFile>, Error> {
        Err(Error::badf())
    }

    async fn datasync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn sync(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn get_filetype(&self) -> Result<wasi_common::file::FileType, Error> {
        Ok(FileType::Pipe)
    }

    async fn get_fdflags(&self) -> Result<wasi_common::file::FdFlags, Error> {
        Ok(FdFlags::empty())
    }

    async fn set_fdflags(&mut self, _flags: wasi_common::file::FdFlags) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn get_filestat(&self) -> Result<wasi_common::file::Filestat, Error> {
        Err(Error::badf())
    }

    async fn set_filestat_size(&self, _size: u64) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn advise(
        &self,
        _offset: u64,
        _len: u64,
        _advice: wasi_common::file::Advice,
    ) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn allocate(&self, _offset: u64, _len: u64) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn set_times(
        &self,
        _atime: Option<wasi_common::clocks::SystemTimeSpec>,
        _mtime: Option<wasi_common::clocks::SystemTimeSpec>,
    ) -> Result<(), Error> {
        Err(Error::badf())
    }

    async fn read_vectored<'a>(&self, bufs: &mut [IoSliceMut<'a>]) -> Result<u64, Error> {
        let inner = self.inner.lock().await;
        let mut p = 0;

        for buf in bufs {
            if p >= inner.buf_len {
                break;
            }

            let sz = (inner.buf_len - p).min(buf.len());
            buf.copy_from_slice(&inner.buf[p..p + sz]);
            p += sz;
        }

        Ok(p as u64)
    }

    async fn read_vectored_at<'a>(
        &self,
        _bufs: &mut [IoSliceMut<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write_vectored<'a>(&self, _bufs: &[IoSlice<'a>]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn write_vectored_at<'a>(
        &self,
        _bufs: &[IoSlice<'a>],
        _offset: u64,
    ) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn seek(&self, _pos: SeekFrom) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn peek(&self, _buf: &mut [u8]) -> Result<u64, Error> {
        Err(Error::badf())
    }

    async fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(self.inner.lock().await.buf_len as u64)
    }

    fn isatty(&self) -> bool {
        false
    }

    async fn readable(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().await;
        let sz = inner.read(&mut inner.buf).await?;
        inner.buf_len = sz;
        Ok(())
    }

    async fn writable(&self) -> Result<(), Error> {
        Err(Error::badf())
    }
}
