use core::pin::pin;
use std::sync::Arc;
use std::net::UdpSocket;
use crate::hardware::{DeviceHardware, razer::HidDeviceManager};

use embassy_futures::select::select4;
use rs_matter::crypto::RngCore;

use rs_matter::crypto::{default_crypto, Crypto};
use rs_matter::dm::clusters::app::level_control::{self, LevelControlHooks};
use rs_matter::dm::clusters::app::color_control::{
    self, ColorControlHooks,
};
use rs_matter::dm::clusters::app::on_off::{
    self, OnOffHooks,
};
use rs_matter::dm::clusters::desc::{self, ClusterHandler as _};
use rs_matter::dm::clusters::groups::{self, ClusterHandler as _};

use rs_matter::dm::clusters::decl::bridged_device_basic_information;
use rs_matter::dm::devices::test::{DAC_PRIVKEY, TEST_DEV_ATT, TEST_DEV_COMM};
use rs_matter::dm::devices::DEV_TYPE_EXTENDED_COLOR_LIGHT;
use rs_matter::dm::DeviceType;

pub const DEV_TYPE_AGGREGATOR: DeviceType = DeviceType {
    dtype: 0x000E,
    drev: 1,
};

pub const DEV_TYPE_BRIDGED_NODE: DeviceType = DeviceType {
    dtype: 0x0013,
    drev: 1,
};

use rs_matter::dm::endpoints;
use rs_matter::dm::networks::eth::EthNetwork;
use rs_matter::dm::networks::SysNetifs;
use rs_matter::dm::{Async, DataModel, Dataver, Endpoint, EpClMatcher, Node};
use rs_matter::im::{EthInteractionModelState, InteractionModel};
use rs_matter::pairing::qr::QrTextType;
use rs_matter::pairing::DiscoveryCapabilities;
use rs_matter::persist::DirKvBlobStore;
use rs_matter::respond::DefaultResponder;
use rs_matter::sc::pase::MAX_COMM_WINDOW_TIMEOUT_SECS;
use rs_matter::tlv::Nullable;
use rs_matter::transport::exchange::MatterBuffers;
use rs_matter::transport::MATTER_SOCKET_BIND_ADDR;
use rs_matter::utils::select::Coalesce;
use rs_matter::{clusters, devices, root_endpoint, Matter, MATTER_PORT};

use super::basic_info::BridgedDeviceBasicInfoHandler;
use super::device_logic::RazerDeviceLogic;
use crate::mdns;

const NODE: Node<'static> = Node {
    endpoints: &[
        root_endpoint!(eth),
        Endpoint::new(
            1, // Bridge / Aggregator Endpoint
            devices!(DEV_TYPE_AGGREGATOR),
            clusters!(
                desc::DescHandler::CLUSTER
            ),
        ),
        Endpoint::new(
            2, // Dock Endpoint
            devices!(DEV_TYPE_BRIDGED_NODE, DEV_TYPE_EXTENDED_COLOR_LIGHT),
            clusters!(
                <BridgedDeviceBasicInfoHandler as bridged_device_basic_information::ClusterHandler>::CLUSTER,
                desc::DescHandler::CLUSTER,
                groups::GroupsHandler::CLUSTER,
                <RazerDeviceLogic as OnOffHooks>::CLUSTER,
                <RazerDeviceLogic as LevelControlHooks>::CLUSTER,
                <RazerDeviceLogic as ColorControlHooks>::CLUSTER
            ),
        ),
        Endpoint::new(
            3, // Keyboard Endpoint
            devices!(DEV_TYPE_BRIDGED_NODE, DEV_TYPE_EXTENDED_COLOR_LIGHT),
            clusters!(
                <BridgedDeviceBasicInfoHandler as bridged_device_basic_information::ClusterHandler>::CLUSTER,
                desc::DescHandler::CLUSTER,
                groups::GroupsHandler::CLUSTER,
                <RazerDeviceLogic as OnOffHooks>::CLUSTER,
                <RazerDeviceLogic as LevelControlHooks>::CLUSTER,
                <RazerDeviceLogic as ColorControlHooks>::CLUSTER
            ),
        ),
    ],
};

fn data_model<'a, OH: OnOffHooks, LH: LevelControlHooks, CH: ColorControlHooks>(
    mut rand: impl RngCore + Copy,
    dock_basic_info: &'a BridgedDeviceBasicInfoHandler,
    dock_on_off: &'a on_off::OnOffHandler<'a, OH, LH>,
    dock_level_control: &'a level_control::LevelControlHandler<'a, LH, OH>,
    dock_color_control: &'a color_control::ColorControlHandler<'a, CH, OH, LH>,
    kbd_basic_info: &'a BridgedDeviceBasicInfoHandler,
    kbd_on_off: &'a on_off::OnOffHandler<'a, OH, LH>,
    kbd_level_control: &'a level_control::LevelControlHandler<'a, LH, OH>,
    kbd_color_control: &'a color_control::ColorControlHandler<'a, CH, OH, LH>,
) -> impl DataModel + 'a {
    (
        NODE,
        endpoints::EthSysHandlerBuilder::new()
            .netif_diag(&SysNetifs)
            .build(rand)
            // Endpoint 1: Bridge / Aggregator
            .chain(
                EpClMatcher::new(Some(1), Some(desc::DescHandler::CLUSTER.id)),
                Async(desc::DescHandler::new_aggregator(Dataver::new_rand(&mut rand)).adapt()),
            )
            // Endpoint 2: Dock
            .chain(
                EpClMatcher::new(Some(2), Some(<BridgedDeviceBasicInfoHandler as bridged_device_basic_information::ClusterHandler>::CLUSTER.id)),
                Async(bridged_device_basic_information::HandlerAdaptor(dock_basic_info)),
            )
            .chain(
                EpClMatcher::new(Some(2), Some(desc::DescHandler::CLUSTER.id)),
                Async(desc::DescHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(2), Some(groups::GroupsHandler::CLUSTER.id)),
                Async(groups::GroupsHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(2), Some(<RazerDeviceLogic as OnOffHooks>::CLUSTER.id)),
                on_off::HandlerAsyncAdaptor(dock_on_off),
            )
            .chain(
                EpClMatcher::new(Some(2), Some(<RazerDeviceLogic as LevelControlHooks>::CLUSTER.id)),
                level_control::HandlerAsyncAdaptor(dock_level_control),
            )
            .chain(
                EpClMatcher::new(Some(2), Some(<RazerDeviceLogic as ColorControlHooks>::CLUSTER.id)),
                color_control::HandlerAsyncAdaptor(dock_color_control),
            )
            // Endpoint 3: Keyboard
            .chain(
                EpClMatcher::new(Some(3), Some(<BridgedDeviceBasicInfoHandler as bridged_device_basic_information::ClusterHandler>::CLUSTER.id)),
                Async(bridged_device_basic_information::HandlerAdaptor(kbd_basic_info)),
            )
            .chain(
                EpClMatcher::new(Some(3), Some(desc::DescHandler::CLUSTER.id)),
                Async(desc::DescHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(3), Some(groups::GroupsHandler::CLUSTER.id)),
                Async(groups::GroupsHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(3), Some(<RazerDeviceLogic as OnOffHooks>::CLUSTER.id)),
                on_off::HandlerAsyncAdaptor(kbd_on_off),
            )
            .chain(
                EpClMatcher::new(Some(3), Some(<RazerDeviceLogic as LevelControlHooks>::CLUSTER.id)),
                level_control::HandlerAsyncAdaptor(kbd_level_control),
            )
            .chain(
                EpClMatcher::new(Some(3), Some(<RazerDeviceLogic as ColorControlHooks>::CLUSTER.id)),
                color_control::HandlerAsyncAdaptor(kbd_color_control),
            ),
    )
}

use rs_matter::dm::clusters::basic_info::BasicInfoConfig;
use rs_matter::dm::devices::test::{TEST_VID, TEST_PID};

pub const MY_DEV_DET: BasicInfoConfig = BasicInfoConfig {
    vid: TEST_VID,
    pid: TEST_PID,
    hw_ver: 1,
    hw_ver_str: "1",
    sw_ver: 1,
    sw_ver_str: "1",
    serial_no: "123456789",
    product_name: "razermatter",
    vendor_name: "Razer",
    device_name: "razermatter",
    ..BasicInfoConfig::new()
};

pub fn run_server() -> Result<(), rs_matter::error::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let matter = Matter::new(&MY_DEV_DET, TEST_DEV_COMM, &TEST_DEV_ATT, MATTER_PORT);

    let store = DirKvBlobStore::new_default();
    let buffers: MatterBuffers = MatterBuffers::new();
    let state: EthInteractionModelState = EthInteractionModelState::new(EthNetwork::new_default());
    let kv = matter.kv(store);

    futures_lite::future::block_on(matter.load_persist(&kv))?;

    let crypto = default_crypto(rand::thread_rng(), DAC_PRIVKEY);
    let mut rand = crypto.rand()?;

    let hardware: Arc<dyn DeviceHardware> = Arc::new(HidDeviceManager::new());
    
    // Handlers for Dock (Endpoint 2)
    let dock_logic = RazerDeviceLogic::new(crate::hardware::razer::DOCK_PID, 0x1F, 0x00, hardware.clone());
    let dock_basic_info = BridgedDeviceBasicInfoHandler::new(Dataver::new_rand(&mut rand), "Razer Thunderbolt 4 Dock");
    
    let dock_on_off_handler = on_off::OnOffHandler::new(Dataver::new_rand(&mut rand), 2, dock_logic.clone());
    let dock_level_control_handler = level_control::LevelControlHandler::new(
        Dataver::new_rand(&mut rand),
        2,
        dock_logic.clone(),
        level_control::AttributeDefaults {
            on_level: Nullable::some(254),
            options: level_control::OptionsBitmap::from_bits(level_control::OptionsBitmap::EXECUTE_IF_OFF.bits()).unwrap(),
            ..Default::default()
        },
    );
    let dock_color_control_handler = color_control::ColorControlHandler::new(
        Dataver::new_rand(&mut rand), 2, dock_logic.clone(), color_control::AttributeDefaults::default(),
    );

    dock_on_off_handler.init(Some(&dock_level_control_handler));
    dock_level_control_handler.init(Some(&dock_on_off_handler));
    dock_color_control_handler.init(Some(&dock_on_off_handler));

    // Handlers for Keyboard (Endpoint 3)
    let kbd_logic = RazerDeviceLogic::new(crate::hardware::razer::KBD_PID, 0x3F, 0x05, hardware.clone());
    let kbd_basic_info = BridgedDeviceBasicInfoHandler::new(Dataver::new_rand(&mut rand), "Razer Huntsman TE Keyboard");
    
    let kbd_on_off_handler = on_off::OnOffHandler::new(Dataver::new_rand(&mut rand), 3, kbd_logic.clone());
    let kbd_level_control_handler = level_control::LevelControlHandler::new(
        Dataver::new_rand(&mut rand),
        3,
        kbd_logic.clone(),
        level_control::AttributeDefaults {
            on_level: Nullable::some(254),
            options: level_control::OptionsBitmap::from_bits(level_control::OptionsBitmap::EXECUTE_IF_OFF.bits()).unwrap(),
            ..Default::default()
        },
    );
    let kbd_color_control_handler = color_control::ColorControlHandler::new(
        Dataver::new_rand(&mut rand), 3, kbd_logic.clone(), color_control::AttributeDefaults::default(),
    );

    kbd_on_off_handler.init(Some(&kbd_level_control_handler));
    kbd_level_control_handler.init(Some(&kbd_on_off_handler));
    kbd_color_control_handler.init(Some(&kbd_on_off_handler));

    let im = InteractionModel::new(
        &matter,
        &crypto,
        &buffers,
        data_model(
            rand,
            &dock_basic_info,
            &dock_on_off_handler,
            &dock_level_control_handler,
            &dock_color_control_handler,
            &kbd_basic_info,
            &kbd_on_off_handler,
            &kbd_level_control_handler,
            &kbd_color_control_handler,
        ),
        &kv,
        &state,
    );

    let responder = DefaultResponder::new(&im);
    let mut respond = pin!(responder.run::<4, 4>());
    let mut im_job = pin!(im.run());

    let socket = async_io::Async::<UdpSocket>::bind(MATTER_SOCKET_BIND_ADDR)?;

    let mut mdns = pin!(mdns::run_mdns(&matter, &crypto));
    let mut transport = pin!(matter.run(&crypto, &socket, &socket, &socket));

    if !matter.is_commissioned() {
        matter.print_standard_qr_text(DiscoveryCapabilities::IP)?;
        matter.print_standard_qr_code(QrTextType::Unicode, DiscoveryCapabilities::IP)?;
        matter.open_basic_comm_window(MAX_COMM_WINDOW_TIMEOUT_SECS, &crypto, &())?;
    }

    let all = select4(&mut transport, &mut mdns, &mut respond, &mut im_job).coalesce();
    futures_lite::future::block_on(all)
}
