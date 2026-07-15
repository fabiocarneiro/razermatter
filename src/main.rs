use core::pin::pin;
use std::sync::{Arc, Mutex};
use std::net::UdpSocket;

use embassy_futures::select::select4;
use rs_matter::crypto::RngCore;

use rs_matter::crypto::{default_crypto, Crypto};
use rs_matter::dm::clusters::app::level_control::{self, LevelControlHooks};
use rs_matter::dm::clusters::app::color_control::{
    self, ColorCapabilitiesBitmap, ColorControlHooks, SetDeviceColor,
};
use rs_matter::dm::clusters::app::on_off::{
    self, EffectVariantEnum, OnOffHooks, StartUpOnOffEnum,
};
use rs_matter::dm::clusters::decl::on_off as on_off_cluster;
use rs_matter::dm::clusters::decl::level_control as level_control_cluster;
use rs_matter::dm::clusters::decl::color_control as color_control_cluster;

use rs_matter::dm::clusters::desc::{self, ClusterHandler as _};
use rs_matter::dm::clusters::groups::{self, ClusterHandler as _};
use rs_matter::dm::devices::test::{DAC_PRIVKEY, TEST_DEV_ATT, TEST_DEV_COMM, TEST_DEV_DET};
use rs_matter::dm::devices::DEV_TYPE_EXTENDED_COLOR_LIGHT;
use rs_matter::dm::endpoints;
use rs_matter::dm::networks::eth::EthNetwork;
use rs_matter::dm::networks::SysNetifs;
use rs_matter::dm::{Async, Cluster, DataModel, Dataver, Endpoint, EpClMatcher, Node};
use rs_matter::error::Error;
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
use rs_matter::{clusters, devices, root_endpoint, with, Matter, MATTER_PORT};

mod mdns;
mod razer;

struct RazerOnOffState {
    on_off: bool,
    start_up_on_off: Option<StartUpOnOffEnum>,
    current_level: Option<u8>,
    start_up_current_level: Option<u8>,
    start_up_color_temperature_mireds: Option<u16>,
}

#[derive(Clone)]
struct RazerDockLogic {
    state: Arc<Mutex<RazerOnOffState>>,
}

impl RazerDockLogic {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RazerOnOffState {
                on_off: false,
                start_up_on_off: None,
                current_level: Some(254),
                start_up_current_level: None,
                start_up_color_temperature_mireds: None,
            })),
        }
    }
}

impl OnOffHooks for RazerDockLogic {
    const CLUSTER: Cluster<'static> = on_off_cluster::FULL_CLUSTER
        .with_revision(6)
        .with_features(on_off_cluster::Feature::LIGHTING.bits())
        .with_attrs(with!(
            required;
            on_off_cluster::AttributeId::OnOff
                | on_off_cluster::AttributeId::GlobalSceneControl
                | on_off_cluster::AttributeId::OnTime
                | on_off_cluster::AttributeId::OffWaitTime
                | on_off_cluster::AttributeId::StartUpOnOff
        ))
        .with_cmds(with!(
            on_off_cluster::CommandId::Off
                | on_off_cluster::CommandId::On
                | on_off_cluster::CommandId::Toggle
                | on_off_cluster::CommandId::OffWithEffect
                | on_off_cluster::CommandId::OnWithRecallGlobalScene
                | on_off_cluster::CommandId::OnWithTimedOff
        ));

    fn on_off(&self) -> bool {
        self.state.lock().unwrap().on_off
    }

    fn set_on_off(&self, on: bool) {
        self.state.lock().unwrap().on_off = on;
        if let Err(e) = razer::set_dock_lighting(on) {
            log::error!("Failed to set dock lighting: {}", e);
        } else {
            log::info!("Dock lighting set to {}", on);
        }
    }

    fn start_up_on_off(&self) -> Nullable<StartUpOnOffEnum> {
        match self.state.lock().unwrap().start_up_on_off {
            Some(value) => Nullable::some(value),
            None => Nullable::none(),
        }
    }

    fn set_start_up_on_off(&self, value: Nullable<StartUpOnOffEnum>) -> Result<(), Error> {
        self.state.lock().unwrap().start_up_on_off = value.into_option();
        Ok(())
    }

    async fn handle_off_with_effect(&self, _effect: EffectVariantEnum) {}
}

impl LevelControlHooks for RazerDockLogic {
    const MIN_LEVEL: u8 = 1;
    const MAX_LEVEL: u8 = 254;
    const FASTEST_RATE: u8 = 50;
    const CLUSTER: Cluster<'static> = level_control_cluster::FULL_CLUSTER
        .with_revision(6)
        .with_features(level_control_cluster::Feature::ON_OFF.bits())
        .with_attrs(with!(
            required;
            level_control_cluster::AttributeId::CurrentLevel
            | level_control_cluster::AttributeId::MinLevel
            | level_control_cluster::AttributeId::MaxLevel
            | level_control_cluster::AttributeId::OnLevel
            | level_control_cluster::AttributeId::Options
        ))
        .with_cmds(with!(
            level_control_cluster::CommandId::MoveToLevel
                | level_control_cluster::CommandId::Move
                | level_control_cluster::CommandId::Step
                | level_control_cluster::CommandId::Stop
                | level_control_cluster::CommandId::MoveToLevelWithOnOff
                | level_control_cluster::CommandId::MoveWithOnOff
                | level_control_cluster::CommandId::StepWithOnOff
                | level_control_cluster::CommandId::StopWithOnOff
        ));

    fn set_device_level(&self, level: u8) -> Result<Option<u8>, ()> {
        if let Err(e) = razer::set_dock_brightness(level) {
            log::error!("Failed to set dock brightness: {}", e);
        } else {
            log::info!("Dock brightness set to {}", level);
        }
        Ok(Some(level))
    }

    fn current_level(&self) -> Option<u8> {
        self.state.lock().unwrap().current_level
    }

    fn set_current_level(&self, level: Option<u8>) {
        self.state.lock().unwrap().current_level = level;
    }

    fn start_up_current_level(&self) -> Result<Option<u8>, Error> {
        Ok(self.state.lock().unwrap().start_up_current_level)
    }

    fn set_start_up_current_level(&self, value: Option<u8>) -> Result<(), Error> {
        self.state.lock().unwrap().start_up_current_level = value;
        Ok(())
    }
}

impl ColorControlHooks for RazerDockLogic {
    const CLUSTER: Cluster<'static> = color_control_cluster::FULL_CLUSTER
        .with_features(
            color_control_cluster::Feature::HUE_AND_SATURATION.bits()
                | color_control_cluster::Feature::ENHANCED_HUE.bits()
                | color_control_cluster::Feature::COLOR_LOOP.bits()
                | color_control_cluster::Feature::XY.bits()
                | color_control_cluster::Feature::COLOR_TEMPERATURE.bits(),
        )
        .with_attrs(with!(
            required;
            color_control_cluster::AttributeId::CurrentHue
                | color_control_cluster::AttributeId::CurrentSaturation
                | color_control_cluster::AttributeId::RemainingTime
                | color_control_cluster::AttributeId::CurrentX
                | color_control_cluster::AttributeId::CurrentY
                | color_control_cluster::AttributeId::ColorTemperatureMireds
                | color_control_cluster::AttributeId::ColorMode
                | color_control_cluster::AttributeId::Options
                | color_control_cluster::AttributeId::NumberOfPrimaries
                | color_control_cluster::AttributeId::EnhancedCurrentHue
                | color_control_cluster::AttributeId::EnhancedColorMode
                | color_control_cluster::AttributeId::ColorLoopActive
                | color_control_cluster::AttributeId::ColorLoopDirection
                | color_control_cluster::AttributeId::ColorLoopTime
                | color_control_cluster::AttributeId::ColorLoopStartEnhancedHue
                | color_control_cluster::AttributeId::ColorLoopStoredEnhancedHue
                | color_control_cluster::AttributeId::ColorCapabilities
                | color_control_cluster::AttributeId::ColorTempPhysicalMinMireds
                | color_control_cluster::AttributeId::ColorTempPhysicalMaxMireds
                | color_control_cluster::AttributeId::CoupleColorTempToLevelMinMireds
                | color_control_cluster::AttributeId::StartUpColorTemperatureMireds
        ))
        .with_cmds(with!(
            color_control_cluster::CommandId::MoveToHue
                | color_control_cluster::CommandId::MoveHue
                | color_control_cluster::CommandId::StepHue
                | color_control_cluster::CommandId::MoveToSaturation
                | color_control_cluster::CommandId::MoveSaturation
                | color_control_cluster::CommandId::StepSaturation
                | color_control_cluster::CommandId::MoveToHueAndSaturation
                | color_control_cluster::CommandId::MoveToColor
                | color_control_cluster::CommandId::MoveColor
                | color_control_cluster::CommandId::StepColor
                | color_control_cluster::CommandId::MoveToColorTemperature
                | color_control_cluster::CommandId::EnhancedMoveToHue
                | color_control_cluster::CommandId::EnhancedMoveHue
                | color_control_cluster::CommandId::EnhancedStepHue
                | color_control_cluster::CommandId::EnhancedMoveToHueAndSaturation
                | color_control_cluster::CommandId::ColorLoopSet
                | color_control_cluster::CommandId::StopMoveStep
                | color_control_cluster::CommandId::MoveColorTemperature
                | color_control_cluster::CommandId::StepColorTemperature
        ));

    const COLOR_CAPABILITIES: ColorCapabilitiesBitmap =
        ColorCapabilitiesBitmap::from_bits_truncate(
            ColorCapabilitiesBitmap::HUE_SATURATION.bits()
                | ColorCapabilitiesBitmap::ENHANCED_HUE.bits()
                | ColorCapabilitiesBitmap::COLOR_LOOP.bits()
                | ColorCapabilitiesBitmap::XY.bits()
                | ColorCapabilitiesBitmap::COLOR_TEMPERATURE.bits(),
        );

    const COLOR_TEMP_PHYSICAL_MIN_MIREDS: u16 = 153;
    const COLOR_TEMP_PHYSICAL_MAX_MIREDS: u16 = 500;
    const COUPLE_COLOR_TEMP_TO_LEVEL_MIN_MIREDS: u16 = Self::COLOR_TEMP_PHYSICAL_MIN_MIREDS;

    fn set_device_color(&self, target: SetDeviceColor) -> Result<(), ()> {
        let (r, g, b) = target.to_rgb(rs_matter::dm::clusters::app::color_control::RgbGamma::Linear);
        if let Err(e) = razer::set_dock_color(r, g, b) {
            log::error!("Failed to set dock color: {}", e);
        } else {
            log::info!("Dock color set to RGB({}, {}, {})", r, g, b);
        }
        Ok(())
    }

    fn start_up_color_temperature_mireds(&self) -> Result<Nullable<u16>, Error> {
        Ok(match self.state.lock().unwrap().start_up_color_temperature_mireds {
            Some(v) => Nullable::some(v),
            None => Nullable::none(),
        })
    }

    fn set_start_up_color_temperature_mireds(&self, value: Nullable<u16>) -> Result<(), Error> {
        self.state.lock().unwrap().start_up_color_temperature_mireds = value.into_option();
        Ok(())
    }
}

const NODE: Node<'static> = Node {
    endpoints: &[
        root_endpoint!(eth),
        Endpoint::new(
            1,
            devices!(DEV_TYPE_EXTENDED_COLOR_LIGHT),
            clusters!(
                desc::DescHandler::CLUSTER,
                groups::GroupsHandler::CLUSTER,
                <RazerDockLogic as OnOffHooks>::CLUSTER,
                <RazerDockLogic as LevelControlHooks>::CLUSTER,
                <RazerDockLogic as ColorControlHooks>::CLUSTER
            ),
        ),
    ],
};

fn data_model<'a, OH: OnOffHooks, LH: LevelControlHooks, CH: ColorControlHooks>(
    mut rand: impl RngCore + Copy,
    on_off: &'a on_off::OnOffHandler<'a, OH, LH>,
    level_control: &'a level_control::LevelControlHandler<'a, LH, OH>,
    color_control: &'a color_control::ColorControlHandler<'a, CH, OH, LH>,
) -> impl DataModel + 'a {
    (
        NODE,
        endpoints::EthSysHandlerBuilder::new()
            .netif_diag(&SysNetifs)
            .build(rand)
            .chain(
                EpClMatcher::new(Some(1), Some(desc::DescHandler::CLUSTER.id)),
                Async(desc::DescHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(1), Some(groups::GroupsHandler::CLUSTER.id)),
                Async(groups::GroupsHandler::new(Dataver::new_rand(&mut rand)).adapt()),
            )
            .chain(
                EpClMatcher::new(Some(1), Some(<RazerDockLogic as OnOffHooks>::CLUSTER.id)),
                on_off::HandlerAsyncAdaptor(on_off),
            )
            .chain(
                EpClMatcher::new(Some(1), Some(<RazerDockLogic as LevelControlHooks>::CLUSTER.id)),
                level_control::HandlerAsyncAdaptor(level_control),
            )
            .chain(
                EpClMatcher::new(Some(1), Some(<RazerDockLogic as ColorControlHooks>::CLUSTER.id)),
                color_control::HandlerAsyncAdaptor(color_control),
            ),
    )
}

fn main() -> Result<(), Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let matter = Matter::new(&TEST_DEV_DET, TEST_DEV_COMM, &TEST_DEV_ATT, MATTER_PORT);

    let store = DirKvBlobStore::new_default();
    let buffers: MatterBuffers = MatterBuffers::new();
    let state: EthInteractionModelState = EthInteractionModelState::new(EthNetwork::new_default());
    let kv = matter.kv(store);

    futures_lite::future::block_on(matter.load_persist(&kv))?;

    let crypto = default_crypto(rand::thread_rng(), DAC_PRIVKEY);
    let mut rand = crypto.rand()?;

    // Handlers
    let razer_logic = RazerDockLogic::new();
    
    let on_off_handler = on_off::OnOffHandler::new(
        Dataver::new_rand(&mut rand),
        1,
        razer_logic.clone(),
    );

    let level_control_handler = level_control::LevelControlHandler::new(
        Dataver::new_rand(&mut rand),
        1,
        razer_logic.clone(),
        level_control::AttributeDefaults {
            on_level: Nullable::some(254),
            options: level_control::OptionsBitmap::from_bits(level_control::OptionsBitmap::EXECUTE_IF_OFF.bits()).unwrap(),
            ..Default::default()
        },
    );

    let color_control_handler = color_control::ColorControlHandler::new(
        Dataver::new_rand(&mut rand),
        1,
        razer_logic.clone(),
        color_control::AttributeDefaults::default(),
    );

    on_off_handler.init(Some(&level_control_handler));
    level_control_handler.init(Some(&on_off_handler));
    color_control_handler.init(Some(&on_off_handler));

    let im = InteractionModel::new(
        &matter,
        &crypto,
        &buffers,
        data_model(rand, &on_off_handler, &level_control_handler, &color_control_handler),
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
