use std::sync::{Arc, Mutex};
use crate::hardware::DeviceHardware;
use crate::protocol::razer::RazerPayload;


use rs_matter::dm::clusters::app::level_control::LevelControlHooks;
use rs_matter::dm::clusters::app::color_control::{
    ColorCapabilitiesBitmap, ColorControlHooks, SetDeviceColor,
};
use rs_matter::dm::clusters::app::on_off::{
    EffectVariantEnum, OnOffHooks, StartUpOnOffEnum,
};
use rs_matter::dm::clusters::decl::on_off as on_off_cluster;
use rs_matter::dm::clusters::decl::level_control as level_control_cluster;
use rs_matter::dm::clusters::decl::color_control as color_control_cluster;
use rs_matter::dm::DeviceType;

pub const DEV_TYPE_AGGREGATOR: DeviceType = DeviceType {
    dtype: 0x000E,
    drev: 1,
};

pub const DEV_TYPE_BRIDGED_NODE: DeviceType = DeviceType {
    dtype: 0x0013,
    drev: 1,
};

use rs_matter::dm::Cluster;
use rs_matter::error::Error;
use rs_matter::tlv::Nullable;
use rs_matter::with;

pub struct RazerOnOffState {
    on_off: bool,
    start_up_on_off: Option<StartUpOnOffEnum>,
    current_level: Option<u8>,
    start_up_current_level: Option<u8>,
    start_up_color_temperature_mireds: Option<u16>,
}

#[derive(Clone)]
pub struct RazerDeviceLogic {
    pid: u16,
    transaction_id: u8,
    led_id: u8,
    state: Arc<Mutex<RazerOnOffState>>,
    hardware: Arc<dyn DeviceHardware>,
}

impl RazerDeviceLogic {
    pub fn new(pid: u16, transaction_id: u8, led_id: u8, hardware: Arc<dyn DeviceHardware>) -> Self {
        Self {
            pid,
            transaction_id,
            led_id,
            state: Arc::new(Mutex::new(RazerOnOffState {
                on_off: false,
                start_up_on_off: None,
                current_level: Some(254),
                start_up_current_level: None,
                start_up_color_temperature_mireds: None,
            })),
            hardware,
        }
    }
}

impl OnOffHooks for RazerDeviceLogic {
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
        let payload = RazerPayload::new_brightness(self.transaction_id, self.led_id, if on { 255 } else { 0 });
        if let Err(e) = self.hardware.send_report(self.pid, &payload.data) {
            log::error!("Failed to set lighting (PID: 0x{:04X}): {}", self.pid, e);
        } else {
            log::info!("Lighting set to {} (PID: 0x{:04X})", on, self.pid);
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

impl LevelControlHooks for RazerDeviceLogic {
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
        let payload = RazerPayload::new_brightness(self.transaction_id, self.led_id, level);
        if let Err(e) = self.hardware.send_report(self.pid, &payload.data) {
            log::error!("Failed to set brightness (PID: 0x{:04X}): {}", self.pid, e);
        } else {
            log::info!("Brightness set to {} (PID: 0x{:04X})", level, self.pid);
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

impl ColorControlHooks for RazerDeviceLogic {
    const CLUSTER: Cluster<'static> = color_control_cluster::FULL_CLUSTER
        .with_features(
            color_control_cluster::Feature::HUE_AND_SATURATION.bits()
                | color_control_cluster::Feature::ENHANCED_HUE.bits()
                | color_control_cluster::Feature::COLOR_LOOP.bits()
                | color_control_cluster::Feature::XY.bits(),
        )
        .with_attrs(with!(
            required;
            color_control_cluster::AttributeId::CurrentHue
                | color_control_cluster::AttributeId::CurrentSaturation
                | color_control_cluster::AttributeId::RemainingTime
                | color_control_cluster::AttributeId::CurrentX
                | color_control_cluster::AttributeId::CurrentY
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
                | color_control_cluster::CommandId::EnhancedMoveToHue
                | color_control_cluster::CommandId::EnhancedMoveHue
                | color_control_cluster::CommandId::EnhancedStepHue
                | color_control_cluster::CommandId::EnhancedMoveToHueAndSaturation
                | color_control_cluster::CommandId::ColorLoopSet
                | color_control_cluster::CommandId::StopMoveStep
        ));

    const COLOR_CAPABILITIES: ColorCapabilitiesBitmap =
        ColorCapabilitiesBitmap::from_bits_truncate(
            ColorCapabilitiesBitmap::HUE_SATURATION.bits()
                | ColorCapabilitiesBitmap::ENHANCED_HUE.bits()
                | ColorCapabilitiesBitmap::COLOR_LOOP.bits()
                | ColorCapabilitiesBitmap::XY.bits(),
        );

    const COLOR_TEMP_PHYSICAL_MIN_MIREDS: u16 = 153;
    const COLOR_TEMP_PHYSICAL_MAX_MIREDS: u16 = 500;
    const COUPLE_COLOR_TEMP_TO_LEVEL_MIN_MIREDS: u16 = Self::COLOR_TEMP_PHYSICAL_MIN_MIREDS;

    fn set_device_color(&self, target: SetDeviceColor) -> Result<(), ()> {
        let (r, g, b) = target.to_rgb(rs_matter::dm::clusters::app::color_control::RgbGamma::Linear);
        let payload = RazerPayload::new_color(self.transaction_id, self.led_id, r, g, b);
        if let Err(e) = self.hardware.send_report(self.pid, &payload.data) {
            log::error!("Failed to set color (PID: 0x{:04X}): {}", self.pid, e);
        } else {
            log::info!("Color set to RGB({}, {}, {}) (PID: 0x{:04X})", r, g, b, self.pid);
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

