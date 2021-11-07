use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use dbus::{
    arg::RefArg,
    nonblock::{MethodReply, Proxy, SyncConnection},
    strings::{BusName, Path},
};
use futures_core::stream::Stream;

use atspi_codegen::accessible::OrgA11yAtspiAccessible;

pub const TIMEOUT: Duration = Duration::from_secs(1);

pub struct Accessible<'a> {
    proxy: Proxy<'a, Arc<SyncConnection>>,
}

impl<'a> Accessible<'a> {
    const INTERFACE: &'static str = "org.a11y.atspi.Accessible";

    #[inline]
    pub fn new(
        destination: impl Into<BusName<'a>>,
        path: impl Into<Path<'a>>,
        conn: Arc<SyncConnection>,
    ) -> Self {
        Self::with_timeout(destination, path, conn, TIMEOUT)
    }

    pub fn with_timeout(
        destination: impl Into<BusName<'a>>,
        path: impl Into<Path<'a>>,
        conn: Arc<SyncConnection>,
        timeout: Duration,
    ) -> Self {
        Self {
            proxy: Proxy::new(destination, path, timeout, conn),
        }
    }

    pub async fn index_in_parent(&self) -> Result<i32, dbus::Error> {
        let (idx,): (i32,) = self
            .proxy
            .method_call(Self::INTERFACE, "GetIndexInParent", ())
            .await?;
        Ok(idx)
    }

    pub async fn localized_role_name(&self) -> Result<String, dbus::Error> {
        let (idx,): (String,) = self
            .proxy
            .method_call(Self::INTERFACE, "GetLocalizedRoleName", ())
            .await?;
        Ok(idx)
    }

    pub async fn child_at_index(&self, idx: i32) -> Result<Option<Accessible<'a>>, dbus::Error> {
        let (dest, path) = self.proxy.get_child_at_index(idx).await?;
        if dest == "org.a11y.atspi.Registry" && path.as_str().unwrap() == "/org/a11y/atspi/null" {
            Ok(None)
        } else {
            let conn = Arc::clone(&self.proxy.connection);
            Ok(Some(Self::with_timeout(
                dest,
                path,
                conn,
                self.proxy.timeout,
            )))
        }
    }

    pub async fn child_count(&self) -> Result<i32, dbus::Error> {
        self.proxy.child_count().await
    }

    pub async fn children(&self) -> Result<Vec<Accessible<'a>>, dbus::Error> {
        let (children,): (Vec<(String, dbus::Path<'static>)>,) = self
            .proxy
            .method_call(Self::INTERFACE, "GetChildren", ())
            .await?;
        let acc_children: Vec<Accessible<'a>> = children
            .into_iter()
            .map(|(string, path)| {
                Accessible::with_timeout(string, path, Arc::clone(&self.proxy.connection), self.proxy.timeout)
             })
            .collect();
        Ok(acc_children)
    }

    pub async fn name(&self) -> Result<String, dbus::Error> {
        self.proxy.name().await
    }

    pub async fn description(&self) -> Result<String, dbus::Error> {
        self.proxy.description().await
    }
}

pub struct ChildStream<'a, 'b> {
    parent: &'a Accessible<'b>,
    current: i32,
    total: i32,
    retry: bool,
    fut: Option<MethodReply<(String, Path<'static>)>>,
}

impl<'b> Stream for ChildStream<'_, 'b> {
    type Item = Result<Accessible<'b>, dbus::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current >= self.total {
            return Poll::Ready(None);
        }

        let fut = Pin::new(if let Some(ref mut fut) = self.fut {
            fut
        } else {
            self.fut = Some(self.parent.proxy.get_child_at_index(self.current));
            self.fut.as_mut().unwrap()
        });

        let res = match fut.poll(cx) {
            Poll::Ready(r) => r,
            Poll::Pending => return Poll::Pending,
        };
        if res.is_err() && !self.retry {
            self.current += 1;
            self.fut = None;
        }
        Poll::Ready(Some(res.map(|(dest, path)| {
            let conn = Arc::clone(&self.parent.proxy.connection);
            Accessible::new(dest, path, conn)
        })))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.total as _,
            if self.retry {
                None
            } else {
                Some(self.total as _)
            },
        )
    }
}
