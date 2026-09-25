#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::super::topology;
use super::*;

pub(super) fn prepare_checked<'anchor, 'inspection>(
    anchor: &'anchor DestinationAnchor<'inspection>,
    plan: &TreePlan<'_>,
    limits: NameProbeLimits,
    wait: Wait<'_>,
    mut observe: impl FnMut(PrepareStep, &str, Option<&std::fs::File>) -> io::Result<()>,
) -> Result<PreparedDestinationTree<'anchor, 'inspection>, DestinationTreePrepareFailure> {
    wait.check()
        .map_err(DestinationTreePrepareFailure::primary)?;
    anchor
        .probe_tree_names(plan, limits, wait)
        .map_err(DestinationTreePrepareFailure::from_probe)?;
    observe(PrepareStep::AfterRepresentation, "", None)
        .map_err(DestinationTreePrepareFailure::primary)?;
    wait.check()
        .map_err(DestinationTreePrepareFailure::primary)?;
    anchor
        .revalidate(wait)
        .map_err(DestinationTreePrepareFailure::primary)?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        topology::require_empty_handle(anchor.retained_handle(), wait)
            .map_err(DestinationTreePrepareFailure::primary)?;
        observe(
            PrepareStep::BeforeCreate,
            "",
            Some(anchor.retained_handle()),
        )
        .map_err(DestinationTreePrepareFailure::primary)?;
        wait.check()
            .map_err(DestinationTreePrepareFailure::primary)?;
        let inner = match native::PreparedTree::create(
            anchor.retained_handle(),
            plan,
            wait,
            &mut observe,
        ) {
            Ok(tree) => tree,
            Err(error) => {
                return Err(DestinationTreePrepareFailure {
                    primary: Some(error.primary),
                    cleanup: error.cleanup,
                    cleanup_complete: error.cleanup_complete,
                })
            }
        };
        if let Err(error) = observe(PrepareStep::BeforeFinalValidation, "", None) {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = wait.check() {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = anchor.revalidate(wait) {
            return Err(fail_and_cleanup(inner, error));
        }
        if let Err(error) = inner.revalidate(wait) {
            return Err(fail_and_cleanup(inner, error));
        }
        Ok(PreparedDestinationTree { anchor, inner })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (anchor, plan, limits, &mut observe);
        Err(DestinationTreePrepareFailure::primary(io::Error::new(
            io::ErrorKind::Unsupported,
            "prepared destination tree requires qualified native topology",
        )))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn fail_and_cleanup(
    mut inner: native::PreparedTree,
    primary: io::Error,
) -> DestinationTreePrepareFailure {
    let cleanup = inner.rollback();
    DestinationTreePrepareFailure {
        primary: Some(primary),
        cleanup: cleanup.errors,
        cleanup_complete: cleanup.complete,
    }
}
