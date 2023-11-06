//! Physical, Atmospherical and Environmental modelizations
// use log::debug;
use crate::prelude::{Config, Epoch, Mode};

//use map_3d::{deg2rad, ecef2geodetic, Ellipsoid};
use std::collections::HashMap;

use gnss::prelude::SV;

use log::{debug, trace};

mod tropo;
pub use tropo::TropoComponents;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

fn default_sv_clock() -> bool {
    true
}

fn default_sv_tgd() -> bool {
    true
}

fn default_iono() -> bool {
    true
}

fn default_tropo() -> bool {
    true
}

fn default_earth_rot() -> bool {
    false
}

fn default_rel_clock_corr() -> bool {
    false
}

/// Atmospherical, Physical and Environmental modeling
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Modeling {
    #[cfg_attr(feature = "serde", serde(default))]
    pub sv_clock_bias: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub tropo_delay: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub iono_delay: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub sv_total_group_delay: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub earth_rotation: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub relativistic_clock_corr: bool,
}

pub(crate) trait Modelization {
    fn sum_up(&self, sv: SV) -> f64;
    /// Modelize environmental effects and atmospherical biases.
    /// "t": Epoch
    /// "sv": buffer
    /// "lat_ddeg": latitude of the receiver [ddeg]
    /// "alt_above_sea_m": altitude of the receiver above sea level [m]
    /// "cfg": passed solver configuration
    /// "tropo_components": possible source of TropoComponents to override internal model
    fn modelize(
        &mut self,
        t: Epoch,
        sv: Vec<(SV, f64)>,
        lat_ddeg: f64,
        alt_above_sea_m: f64,
        cfg: &Config,
        tropo_components: Option<TropoComponents>,
    );
}

impl Default for Modeling {
    fn default() -> Self {
        Self {
            sv_clock_bias: default_sv_clock(),
            iono_delay: default_iono(),
            tropo_delay: default_tropo(),
            sv_total_group_delay: default_sv_tgd(),
            earth_rotation: default_earth_rot(),
            relativistic_clock_corr: default_rel_clock_corr(),
        }
    }
}

impl From<Mode> for Modeling {
    fn from(mode: Mode) -> Self {
        let mut s = Self::default();
        match mode {
            //TODO
            //Mode::PPP => {
            //    s.earth_rotation = true;
            //    s.relativistic_clock_corr = true;
            //},
            _ => {},
        }
        s
    }
}
pub type Models = HashMap<SV, f64>;

impl Modelization for Models {
    fn modelize(
        &mut self,
        t: Epoch,
        sv: Vec<(SV, f64)>,
        lat_ddeg: f64,
        alt_above_sea_m: f64,
        cfg: &Config,
        tropo_components: Option<TropoComponents>,
    ) {
        self.clear();
        for (sv, elev) in sv {
            self.insert(sv, 0.0_f64);

            if cfg.modeling.tropo_delay {
                let components = match tropo_components {
                    Some(components) => {
                        trace!(
                            "tropo delay (overriden): zwd: {}, zdd: {}",
                            components.zwd,
                            components.zdd
                        );
                        components
                    },
                    None => {
                        let (zdd, zwd) = tropo::unb3_delay_components(t, lat_ddeg, alt_above_sea_m);
                        trace!("unb3 model: zwd: {}, zdd: {}", zdd, zwd);
                        TropoComponents { zwd, zdd }
                    },
                };

                let tropo = tropo::tropo_delay(elev, components.zwd, components.zdd);
                debug!("{:?}: {}(e={:.3}) tropo delay {} [m]", t, sv, elev, tropo);
                self.insert(sv, tropo);
            }
        }
    }
    fn sum_up(&self, sv: SV) -> f64 {
        self.iter()
            .filter_map(|(k, v)| if *k == sv { Some(*v) } else { None })
            .reduce(|k, _| k)
            .unwrap() // unsed in infaillible manner, at main level
    }
}
