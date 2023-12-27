pub mod draw;
pub mod lighting;

impl Into<draw::GPUGlobalData> for lighting::GPUGlobalData {
    fn into(self) -> draw::GPUGlobalData {
        draw::GPUGlobalData {
            view: self.view,
            proj: self.proj,
            view_proj: self.view_proj,
            inv_view_proj: self.inv_view_proj,
        }
    }
}

impl Into<lighting::GPUGlobalData> for draw::GPUGlobalData {
    fn into(self) -> lighting::GPUGlobalData {
        lighting::GPUGlobalData {
            view: self.view,
            proj: self.proj,
            view_proj: self.view_proj,
            inv_view_proj: self.inv_view_proj,
        }
    }
}
