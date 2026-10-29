fn horizon_utilities_pickup(taken: &crate::game::pickups::Taken, collector: crate::game::pickups::Collector) {
    use crate::game::pickups::{Collector, Taken};
    if collector == Collector::Player {
        if let Taken::Item(item) = taken { crate::game::horizon_utilities_native::pickup(&item.stack.item); }
    }
}
