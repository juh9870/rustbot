use std::any::TypeId;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

#[derive(Debug, Default)]
pub struct PoiseResources {
    resources: HashMap<TypeId, Mutex<Box<dyn std::any::Any + Send + Sync>>>,
}

impl PoiseResources {
    /// Initialize a resource of type `T` in the `PoiseData`
    pub fn init_resource<T: 'static + Send + Sync>(&mut self, resource: T) {
        let type_id = TypeId::of::<T>();
        self.resources
            .insert(type_id, Mutex::new(Box::new(resource)));
    }

    /// Get a resource of type `T` from the `PoiseData`
    pub async fn get_resource<T: 'static + Send + Sync>(
        &self,
    ) -> rootcause::Result<MappedMutexGuard<'_, T>> {
        let mu = self.resources.get(&TypeId::of::<T>()).ok_or_else(|| {
            rootcause::report!("Resource {:?} not found", std::any::type_name::<T>())
        })?;

        let guard = mu.lock().await;

        Ok(MutexGuard::map(guard, |boxed| {
            boxed.downcast_mut::<T>().unwrap()
        }))
    }

    /// Execute a closure with a resource of type `T` from the `PoiseData`
    pub async fn with_resource<
        T: 'static + Send + Sync,
        F: FnOnce(&mut T) -> RF,
        RF: Future<Output = rootcause::Result<R>>,
        R,
    >(
        &self,
        f: F,
    ) -> rootcause::Result<R> {
        let mut resource_guard = self.get_resource::<T>().await?;
        let result = f(&mut *resource_guard).await?;
        Ok(result)
    }
}

pub type PoiseContext<'a> = poise::Context<'a, Arc<PoiseResources>, rootcause::Report>;
