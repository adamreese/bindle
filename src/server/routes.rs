use warp::Filter;

/// A helper function that aggregates all routes into a complete API filter. If you only wish to
/// serve specific endpoints or versions, you can assemble them with the individual submodules
pub fn api<P, I>(
    store: P,
    index: I,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
where
    P: crate::provider::Provider + Clone + Send + Sync + 'static,
    I: crate::search::Search + Clone + Send + Sync + 'static,
{
    warp::path("v1").and(
        v1::invoice::query(index)
            .or(v1::invoice::create(store.clone()))
            .or(v1::invoice::get(store.clone()))
            .or(v1::invoice::head(store.clone()))
            .or(v1::invoice::yank(store.clone()))
            .or(v1::parcel::create(store.clone()))
            .or(v1::relationships::get_missing_parcels(store)),
    )
}

pub mod v1 {
    use crate::provider::Provider;
    use crate::search::Search;
    use crate::server::handlers::v1::*;
    use crate::server::{filters, routes::with_store};

    use warp::Filter;

    pub mod invoice {
        use super::*;

        pub fn query<S>(
            index: S,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            S: Search + Clone + Send + Sync,
        {
            warp::path("_q")
                .and(warp::get())
                .and(warp::query::<crate::QueryOptions>())
                .and(warp::any().map(move || index.clone()))
                .and_then(query_invoices)
        }

        pub fn create<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            warp::path("_i")
                .and(warp::path::end())
                .and(warp::post())
                .and(with_store(store))
                .and(filters::toml())
                .and_then(create_invoice)
                .recover(filters::handle_deserialize_rejection)
        }

        // The GET and HEAD endpoints handle both parcels and invoices through the request router function
        pub fn get<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            warp::path("_i")
                .and(warp::path::tail())
                .and(warp::get())
                .and(warp::query::<filters::InvoiceQuery>())
                .and(with_store(store))
                .and(warp::method())
                .and_then(request_router)
        }

        pub fn head<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            warp::path("_i")
                .and(warp::path::tail())
                .and(warp::head())
                .and(warp::query::<filters::InvoiceQuery>())
                .and(with_store(store))
                .and(warp::method())
                .and_then(request_router)
        }

        pub fn yank<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            warp::path("_i")
                .and(warp::path::tail())
                .and(warp::delete())
                .and(with_store(store))
                .and_then(yank_invoice)
        }
    }

    pub mod parcel {
        use super::*;

        pub fn create<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            warp::path("_i")
                .and(warp::path::tail())
                .and(warp::post())
                .and(warp::body::stream())
                .and(with_store(store))
                .and_then(create_parcel)
        }
    }

    pub mod relationships {
        use super::*;

        pub fn get_missing_parcels<P>(
            store: P,
        ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
        where
            P: Provider + Clone + Send + Sync,
        {
            // For some reason, using the `path!` macro here was causing matching problems
            warp::path("_r")
                .and(warp::path("missing"))
                .and(warp::path::tail())
                .and(warp::get())
                .and(with_store(store))
                .and_then(get_missing)
        }
    }
}

pub(crate) fn with_store<P>(
    store: P,
) -> impl Filter<Extract = (P,), Error = std::convert::Infallible> + Clone
where
    P: crate::provider::Provider + Clone + Send,
{
    // We have to clone for this to be Fn instead of FnOnce
    warp::any().map(move || store.clone())
}
