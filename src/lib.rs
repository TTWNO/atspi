use std::{
    sync::Arc,
    time::Duration,
};

use dbus::{
    nonblock::{Proxy, SyncConnection},
    strings::{BusName, Path},
};

use atspi_codegen::accessible::OrgA11yAtspiAccessible;

pub const TIMEOUT: Duration = Duration::from_secs(1);

pub struct Accessible {
    proxy: Proxy<'static, Arc<SyncConnection>>,
}

impl Accessible {
    const INTERFACE: &'static str = "org.a11y.atspi.Accessible";

    #[inline]
    pub fn new(
        destination: impl Into<BusName<'static>>,
        path: impl Into<Path<'static>>,
        conn: Arc<SyncConnection>,
    ) -> Self {
        Self::with_timeout(destination, path, conn, TIMEOUT)
    }

    pub fn with_timeout(
        destination: impl Into<BusName<'static>>,
        path: impl Into<Path<'static>>,
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

pub async fn child_at_index(&self, idx: i32) -> Result<Option<Self>, dbus::Error> {
    let (dest, path) = self.proxy.get_child_at_index(idx).await?;
    if dest == "org.a11y.atspi.Registry" && &*path == "/org/a11y/atspi/null" {
        Ok(None)
    } else {
    let conn = Arc::clone(&self.proxy.connection);
    Ok(Some(Self::with_timeout(dest, path, conn, self.proxy.timeout)))
    }
}

    pub async fn child_count(&self) -> Result<i32, dbus::Error> {
        self.proxy.child_count().await
    }

    pub async fn name(&self) -> Result<String, dbus::Error> {
        self.proxy.name().await
    }

    pub async fn description(&self) -> Result<String, dbus::Error> {
        self.proxy.description().await
    }
}
