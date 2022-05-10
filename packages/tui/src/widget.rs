use euclid::Rect;

use crate::terminal::RegionMask;

pub(crate) trait RinkWidget {
    fn render(self, area: Rect<u16, u16>, buf: &mut RegionMask);
}
