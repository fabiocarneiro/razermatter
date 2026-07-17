use rs_matter::dm::clusters::decl::bridged_device_basic_information::*;
use rs_matter::dm::clusters::decl::bridged_device_basic_information;
use rs_matter::dm::{Cluster, Dataver, ReadContext};
use rs_matter::error::Error;
use rs_matter::tlv::{TLVBuilderParent, Utf8StrBuilder};
use rs_matter::with;

pub struct BridgedDeviceBasicInfoHandler {
    pub name: &'static str,
    dataver: Dataver,
}

impl BridgedDeviceBasicInfoHandler {
    pub const fn new(dataver: Dataver, name: &'static str) -> Self {
        Self { dataver, name }
    }

    pub const fn adapt(self) -> bridged_device_basic_information::HandlerAdaptor<Self> {
        bridged_device_basic_information::HandlerAdaptor(self)
    }
}

impl bridged_device_basic_information::ClusterHandler for BridgedDeviceBasicInfoHandler {
    const CLUSTER: Cluster<'static> = bridged_device_basic_information::FULL_CLUSTER.with_attrs(with!(
        AttributeId::NodeLabel | AttributeId::Reachable | AttributeId::VendorName | AttributeId::ProductName
    ));
    
    fn dataver(&self) -> u32 {
        self.dataver.get()
    }
    
    fn dataver_changed(&self) {
        self.dataver.changed();
    }
    
    fn node_label<P: TLVBuilderParent>(&self, _ctx: impl ReadContext, builder: Utf8StrBuilder<P>) -> Result<P, Error> {
        builder.set(self.name)
    }
    
    fn product_name<P: TLVBuilderParent>(&self, _ctx: impl ReadContext, builder: Utf8StrBuilder<P>) -> Result<P, Error> {
        builder.set(self.name)
    }
    
    fn vendor_name<P: TLVBuilderParent>(&self, _ctx: impl ReadContext, builder: Utf8StrBuilder<P>) -> Result<P, Error> {
        builder.set("Razer")
    }

    fn reachable(&self, _ctx: impl ReadContext) -> Result<bool, Error> {
        Ok(true)
    }

    fn unique_id<P: TLVBuilderParent>(&self, _ctx: impl ReadContext, builder: Utf8StrBuilder<P>) -> Result<P, Error> {
        builder.set("RAZER_BRIDGED")
    }

    fn handle_keep_active(&self, _ctx: impl rs_matter::dm::InvokeContext, _req: bridged_device_basic_information::KeepActiveRequest<'_>) -> Result<(), Error> {
        Ok(())
    }
}
