use std::cell::Cell;

use crate::ElementId;

trait NodeLike {}

trait EventHandlingNode: NodeLike {}

struct Node<NL: NodeLike> {
    id: Cell<Option<ElementId>>,
    parent_id: Cell<Option<ElementId>>,
    inner: NL,
}
