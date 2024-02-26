use legion::world::Entry;

use crate::render::resource_manager::ResourceRetriever;

pub trait ComponentInfo {
    fn add_to_entry(&self, entry: &mut Entry, resources: &mut ResourceRetriever);
}
