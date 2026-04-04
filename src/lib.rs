use serde::{Deserialize, Serialize};

const RD: f64 = 287.04;
const RV: f64 = 461.5;
const PHI: f64 = RD / RV;
const CPD: f64 = 1005.0;
const CPV: f64 = 1870.0;
const CPL: f64 = 4190.0;
const CPI: f64 = 2106.0;
const G: f64 = 9.81;
const P0: f64 = 100000.0;
const KAPPA: f64 = RD / CPD;
const LV_TRIP: f64 = 2_501_000.0;
const LI_TRIP: f64 = 333_000.0;
const T_TRIP: f64 = 273.15;
const VAPOR_PRES_REF: f64 = 611.2;
const MOLAR_GAS_CONSTANT: f64 = 8.314;
const AVG_MOLAR_MASS: f64 = 0.029;
const DEFAULT_STEP_M: f64 = 20.0;
const KELVIN_OFFSET: f64 = 273.15;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapeType {
    SurfaceBased,
    MostUnstable,
    MixedLayer,
    UserDefined,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StormMotionType {
    RightMoving,
    LeftMoving,
    MeanWind,
    UserDefined,
}

impl Default for CapeType {
    fn default() -> Self {
        Self::SurfaceBased
    }
}

impl Default for StormMotionType {
    fn default() -> Self {
        Self::RightMoving
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParcelOptions {
    #[serde(default)]
    pub cape_type: CapeType,
    #[serde(default)]
    pub storm_motion_type: StormMotionType,
    #[serde(default)]
    pub origin_pressure_pa: Option<f64>,
    #[serde(default)]
    pub origin_height_m: Option<f64>,
    #[serde(default)]
    pub mixed_layer_depth_pa: Option<f64>,
    #[serde(default)]
    pub inflow_layer_bottom_m: Option<f64>,
    #[serde(default)]
    pub inflow_layer_top_m: Option<f64>,
    #[serde(default)]
    pub storm_motion_u_ms: Option<f64>,
    #[serde(default)]
    pub storm_motion_v_ms: Option<f64>,
    #[serde(default)]
    pub entrainment_rate: Option<f64>,
    #[serde(default)]
    pub pseudoadiabatic: Option<bool>,
}

impl Default for ParcelOptions {
    fn default() -> Self {
        Self {
            cape_type: CapeType::SurfaceBased,
            storm_motion_type: StormMotionType::RightMoving,
            origin_pressure_pa: None,
            origin_height_m: None,
            mixed_layer_depth_pa: Some(10000.0),
            inflow_layer_bottom_m: Some(0.0),
            inflow_layer_top_m: Some(1000.0),
            storm_motion_u_ms: None,
            storm_motion_v_ms: None,
            entrainment_rate: None,
            pseudoadiabatic: Some(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParcelProfile {
    pub pressure_pa: Vec<f64>,
    pub height_m: Vec<f64>,
    pub temperature_k: Vec<f64>,
    pub qv_kgkg: Vec<f64>,
    pub qt_kgkg: Vec<f64>,
    pub buoyancy_ms2: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapeCinLfcEl {
    pub cape_jkg: f64,
    pub cin_jkg: f64,
    pub lfc_m: Option<f64>,
    pub el_m: Option<f64>,
    pub origin_index: usize,
    pub pressure_pa: Vec<f64>,
    pub height_m: Vec<f64>,
    pub parcel_temperature_k: Vec<f64>,
    pub buoyancy_ms2: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcapeNcape {
    pub ecape_jkg: f64,
    pub ncape_jkg: f64,
    pub cape_jkg: f64,
    pub lfc_m: Option<f64>,
    pub el_m: Option<f64>,
    pub storm_motion_u_ms: f64,
    pub storm_motion_v_ms: f64,
    pub storm_relative_wind_ms: f64,
    pub psi: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcapeParcelResult {
    pub ecape_jkg: f64,
    pub ncape_jkg: f64,
    pub cape_jkg: f64,
    pub cin_jkg: f64,
    pub lfc_m: Option<f64>,
    pub el_m: Option<f64>,
    pub storm_motion_u_ms: f64,
    pub storm_motion_v_ms: f64,
    pub parcel_profile: ParcelProfile,
}

#[derive(Debug, Clone)]
struct ParcelOriginState {
    index: usize,
    theta_override_k: Option<f64>,
    qv_override_kgkg: Option<f64>,
    height_override_m: Option<f64>,
}

#[derive(Debug)]
pub enum EcapeError {
    DimensionMismatch,
    EmptyProfile,
    NonMonotonicPressure,
    NonMonotonicHeight,
    NonFiniteInput,
    OriginNotFound,
}

impl std::fmt::Display for EcapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionMismatch => write!(f, "profile arrays must have the same length"),
            Self::EmptyProfile => write!(f, "profile is empty"),
            Self::NonMonotonicPressure => write!(f, "pressure must decrease monotonically"),
            Self::NonMonotonicHeight => write!(f, "height must increase monotonically"),
            Self::NonFiniteInput => write!(f, "profile contains non-finite values"),
            Self::OriginNotFound => write!(f, "could not determine parcel origin"),
        }
    }
}

impl std::error::Error for EcapeError {}

fn validate_profile(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    qv_kgkg: &[f64],
    u_wind_ms: &[f64],
    v_wind_ms: &[f64],
) -> Result<(), EcapeError> {
    let n = height_m.len();
    if n == 0 {
        return Err(EcapeError::EmptyProfile);
    }
    if pressure_pa.len() != n
        || temperature_k.len() != n
        || qv_kgkg.len() != n
        || u_wind_ms.len() != n
        || v_wind_ms.len() != n
    {
        return Err(EcapeError::DimensionMismatch);
    }
    for i in 0..n {
        if !height_m[i].is_finite()
            || !pressure_pa[i].is_finite()
            || !temperature_k[i].is_finite()
            || !qv_kgkg[i].is_finite()
            || !u_wind_ms[i].is_finite()
            || !v_wind_ms[i].is_finite()
        {
            return Err(EcapeError::NonFiniteInput);
        }
        if i > 0 {
            if height_m[i] <= height_m[i - 1] {
                return Err(EcapeError::NonMonotonicHeight);
            }
            if pressure_pa[i] >= pressure_pa[i - 1] {
                return Err(EcapeError::NonMonotonicPressure);
            }
        }
    }
    Ok(())
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn linear_interp(x0: f64, x1: f64, y0: f64, y1: f64, x: f64) -> f64 {
    if (x1 - x0).abs() < 1e-12 {
        return y0;
    }
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

fn reverse_linear_interp(xs: &[f64], ys: &[f64], target: f64) -> Option<f64> {
    for i in 1..xs.len() {
        let y0 = ys[i - 1];
        let y1 = ys[i];
        if (target >= y0 && target <= y1) || (target >= y1 && target <= y0) {
            return Some(linear_interp(y0, y1, xs[i - 1], xs[i], target));
        }
    }
    None
}

fn find_bracketing_index_desc(values: &[f64], target: f64) -> usize {
    if target >= values[0] {
        return 0;
    }
    for i in 1..values.len() {
        if target >= values[i] {
            return i - 1;
        }
    }
    values.len() - 2
}

fn find_bracketing_index_asc(values: &[f64], target: f64) -> usize {
    if target <= values[0] {
        return 0;
    }
    for i in 1..values.len() {
        if target <= values[i] {
            return i - 1;
        }
    }
    values.len() - 2
}

fn interp_pressure_to_height(heights: &[f64], pressures: &[f64], z: f64) -> f64 {
    let i = find_bracketing_index_asc(heights, z);
    linear_interp(heights[i], heights[i + 1], pressures[i], pressures[i + 1], z)
}

fn interp_height_to_pressure(pressures: &[f64], heights: &[f64], p: f64) -> f64 {
    let i = find_bracketing_index_desc(pressures, p);
    linear_interp(pressures[i], pressures[i + 1], heights[i], heights[i + 1], p)
}

fn interp_profile_at_height(
    heights: &[f64],
    values: &[f64],
    z: f64,
) -> f64 {
    if z <= heights[0] {
        return values[0];
    }
    if z >= heights[heights.len() - 1] {
        return values[values.len() - 1];
    }
    let i = find_bracketing_index_asc(heights, z);
    linear_interp(heights[i], heights[i + 1], values[i], values[i + 1], z)
}

fn saturation_vapor_pressure_water(temp_k: f64) -> f64 {
    let latent_heat = LV_TRIP - (CPL - CPV) * (temp_k - T_TRIP);
    let heat_power = (CPL - CPV) / RV;
    let exp_term = ((LV_TRIP / T_TRIP - latent_heat / temp_k) / RV).exp();
    VAPOR_PRES_REF * (T_TRIP / temp_k).powf(heat_power) * exp_term
}

fn omega(temp_k: f64, warmest_mixed_phase_temp: f64, coldest_mixed_phase_temp: f64) -> f64 {
    if temp_k >= warmest_mixed_phase_temp {
        0.0
    } else if temp_k <= coldest_mixed_phase_temp {
        1.0
    } else {
        (temp_k - warmest_mixed_phase_temp) / (coldest_mixed_phase_temp - warmest_mixed_phase_temp)
    }
}

fn omega_deriv(temp_k: f64, warmest_mixed_phase_temp: f64, coldest_mixed_phase_temp: f64) -> f64 {
    if temp_k >= warmest_mixed_phase_temp || temp_k <= coldest_mixed_phase_temp {
        0.0
    } else {
        1.0 / (coldest_mixed_phase_temp - warmest_mixed_phase_temp)
    }
}

fn r_sat(temp_k: f64, pressure_pa: f64, ice_flag: i32) -> f64 {
    let warm = 273.15;
    let cold = 253.15;
    let omeg = omega(temp_k, warm, cold);
    if ice_flag == 0 {
        let term1 = (CPV - CPL) / RV;
        let term2 = (LV_TRIP - T_TRIP * (CPV - CPL)) / RV;
        let esl =
            ((temp_k - T_TRIP) * term2 / (temp_k * T_TRIP)).exp() * VAPOR_PRES_REF * (temp_k / T_TRIP).powf(term1);
        PHI * esl / (pressure_pa - esl).max(1e-9)
    } else if ice_flag == 1 {
        let qsat_l = r_sat(temp_k, pressure_pa, 0);
        let qsat_i = r_sat(temp_k, pressure_pa, 2);
        (1.0 - omeg) * qsat_l + omeg * qsat_i
    } else {
        let term1 = (CPV - CPI) / RV;
        let term2 = (LV_TRIP - T_TRIP * (CPV - CPI)) / RV;
        let esl =
            ((temp_k - T_TRIP) * term2 / (temp_k * T_TRIP)).exp() * VAPOR_PRES_REF * (temp_k / T_TRIP).powf(term1);
        PHI * esl / (pressure_pa - esl).max(1e-9)
    }
}

fn vapor_pressure_from_specific_humidity(pressure_pa: f64, qv_kgkg: f64) -> f64 {
    pressure_pa * qv_kgkg / (PHI + (1.0 - PHI) * qv_kgkg)
}

fn specific_humidity_from_vapor_pressure(pressure_pa: f64, vapor_pressure_pa: f64) -> f64 {
    PHI * vapor_pressure_pa / (pressure_pa - (1.0 - PHI) * vapor_pressure_pa)
}

fn dewpoint_from_vapor_pressure(vapor_pressure_pa: f64) -> f64 {
    let ln_ratio = (vapor_pressure_pa / 611.2).ln();
    let td_c = 243.5 * ln_ratio / (17.67 - ln_ratio);
    td_c + 273.15
}

fn dewpoint_from_specific_humidity(pressure_pa: f64, qv_kgkg: f64) -> f64 {
    dewpoint_from_vapor_pressure(vapor_pressure_from_specific_humidity(pressure_pa, qv_kgkg))
}

fn specific_humidity_from_dewpoint(pressure_pa: f64, dewpoint_k: f64) -> f64 {
    let vapor_pressure_pa =
        611.2 * ((17.67 * (dewpoint_k - KELVIN_OFFSET)) / (dewpoint_k - 29.65)).exp();
    specific_humidity_from_vapor_pressure(pressure_pa, vapor_pressure_pa)
}

fn potential_temperature(temp_k: f64, pressure_pa: f64) -> f64 {
    temp_k * (P0 / pressure_pa).powf(KAPPA)
}

fn temperature_from_potential_temperature(theta_k: f64, pressure_pa: f64) -> f64 {
    theta_k * (pressure_pa / P0).powf(KAPPA)
}

fn density_temperature(temp_k: f64, qv_kgkg: f64, qt_kgkg: f64) -> f64 {
    temp_k * (1.0 - qt_kgkg + qv_kgkg / PHI)
}

fn equivalent_potential_temperature(temp_k: f64, dewpoint_k: f64, pressure_pa: f64) -> f64 {
    let q = specific_humidity_from_dewpoint(pressure_pa, dewpoint_k);
    let e = 611.2 * ((17.67 * (dewpoint_k - KELVIN_OFFSET)) / ((dewpoint_k - KELVIN_OFFSET) + 243.5)).exp();
    let w = PHI * e / (pressure_pa - e).max(1e-9);
    let tlcl = 1.0 / (1.0 / (dewpoint_k - 56.0).max(1e-6) + ((temp_k / dewpoint_k).max(1e-9)).ln() / 800.0) + 56.0;
    let theta_l = temp_k * (P0 / pressure_pa).powf(0.2854 * (1.0 - 0.28 * q));
    theta_l * (((3376.0 / tlcl) - 2.54) * w * (1.0 + 0.81 * w)).exp()
}

fn unsaturated_adiabatic_lapse_rate(
    temperature_parcel: f64,
    qv_parcel: f64,
    temperature_env: f64,
    qv_env: f64,
    entrainment_rate: f64,
) -> f64 {
    let temperature_entrainment = -entrainment_rate * (temperature_parcel - temperature_env);
    let density_temperature_parcel = density_temperature(temperature_parcel, qv_parcel, qv_parcel);
    let density_temperature_env = density_temperature(temperature_env, qv_env, qv_env);
    let buoyancy = G * (density_temperature_parcel - density_temperature_env) / density_temperature_env;
    let c_pmv = (1.0 - qv_parcel) * CPD + qv_parcel * CPV;
    (-G / CPD) * ((1.0 + buoyancy / G) / (c_pmv / CPD)) + temperature_entrainment
}

fn saturated_adiabatic_lapse_rate(
    temperature_parcel: f64,
    qt_parcel: f64,
    pressure_parcel: f64,
    temperature_env: f64,
    qv_env: f64,
    entrainment_rate: f64,
    prate: f64,
    qt_entrainment: Option<f64>,
) -> f64 {
    let omega = omega(temperature_parcel, 273.15, 253.15);
    let d_omega = omega_deriv(temperature_parcel, 273.15, 253.15);
    let q_vsl = (1.0 - qt_parcel) * r_sat(temperature_parcel, pressure_parcel, 0);
    let q_vsi = (1.0 - qt_parcel) * r_sat(temperature_parcel, pressure_parcel, 2);
    let qv_parcel = (1.0 - omega) * q_vsl + omega * q_vsi;
    let temperature_entrainment = -entrainment_rate * (temperature_parcel - temperature_env);
    let qv_entrainment = -entrainment_rate * (qv_parcel - qv_env);
    let qt_entrainment =
        qt_entrainment.unwrap_or(-entrainment_rate * (qt_parcel - qv_env) - prate * (qt_parcel - qv_parcel));
    let q_condensate = qt_parcel - qv_parcel;
    let ql_parcel = q_condensate * (1.0 - omega);
    let qi_parcel = q_condensate * omega;
    let c_pm = (1.0 - qt_parcel) * CPD + qv_parcel * CPV + ql_parcel * CPL + qi_parcel * CPI;
    let density_temperature_parcel = density_temperature(temperature_parcel, qv_parcel, qt_parcel);
    let density_temperature_env = density_temperature(temperature_env, qv_env, qv_env);
    let buoyancy = G * (density_temperature_parcel - density_temperature_env) / density_temperature_env;
    let l_v = LV_TRIP + (temperature_parcel - T_TRIP) * (CPV - CPL);
    let l_i = LI_TRIP + (temperature_parcel - T_TRIP) * (CPL - CPI);
    let l_s = l_v + omega * l_i;
    let q_vsl_cap = q_vsl / (PHI - PHI * qt_parcel + qv_parcel);
    let q_vsi_cap = q_vsi / (PHI - PHI * qt_parcel + qv_parcel);
    let q_m = (1.0 - omega) * q_vsl / (1.0 - q_vsl_cap) + omega * q_vsi / (1.0 - q_vsi_cap);
    let l_m = (1.0 - omega) * l_v * q_vsl / (1.0 - q_vsl_cap)
        + omega * (l_v + l_i) * q_vsi / (1.0 - q_vsi_cap);
    let r_m0 = (1.0 - qv_env) * RD + qv_env * RV;
    let term_1 = buoyancy;
    let term_2 = G;
    let term_3 = ((l_s * q_m) / (r_m0 * temperature_env)) * G;
    let term_4 = (c_pm - l_i * (qt_parcel - qv_parcel) * d_omega) * temperature_entrainment;
    let term_5 = l_s * (qv_entrainment + qv_parcel / (1.0 - qt_parcel) * qt_entrainment);
    let term_6 = c_pm;
    let term_7 = (l_i * (qt_parcel - qv_parcel) - l_s * (q_vsi - q_vsl)) * d_omega;
    let term_8 = (l_s * l_m) / (RV * temperature_parcel * temperature_parcel);
    -(term_1 + term_2 + term_3 - term_4 - term_5) / (term_6 - term_7 + term_8)
}

fn pressure_at_height(ref_pressure: f64, height_above_ref_pressure: f64, temperature: f64) -> f64 {
    let scale_height = (MOLAR_GAS_CONSTANT * temperature) / (AVG_MOLAR_MASS * G);
    ref_pressure * (-height_above_ref_pressure / scale_height).exp()
}

fn moist_static_energy(z_m: f64, temp_k: f64, qv_kgkg: f64) -> f64 {
    CPD * temp_k + G * z_m + LV_TRIP * qv_kgkg
}

fn layer_mean(values: &[f64], heights: &[f64], bottom: f64, top: f64) -> f64 {
    let mut accum = 0.0;
    let mut weight = 0.0;
    for i in 1..heights.len() {
        let z0 = heights[i - 1].max(bottom);
        let z1 = heights[i].min(top);
        if z1 <= z0 {
            continue;
        }
        let v0 = linear_interp(heights[i - 1], heights[i], values[i - 1], values[i], z0);
        let v1 = linear_interp(heights[i - 1], heights[i], values[i - 1], values[i], z1);
        let dz = z1 - z0;
        accum += 0.5 * (v0 + v1) * dz;
        weight += dz;
    }
    if weight == 0.0 { values[0] } else { accum / weight }
}

fn wind_components_from_direction_speed_scalar(direction_deg: f64, speed: f64) -> (f64, f64) {
    let rad = direction_deg.to_radians();
    (-speed * rad.sin(), -speed * rad.cos())
}

pub fn wind_components_from_direction_speed(direction_deg: f64, speed: f64) -> (f64, f64) {
    wind_components_from_direction_speed_scalar(direction_deg, speed)
}

fn resolve_parcel_origin(
    heights: &[f64],
    pressures: &[f64],
    temperatures: &[f64],
    qv: &[f64],
    options: &ParcelOptions,
) -> Result<ParcelOriginState, EcapeError> {
    if let Some(origin_p) = options.origin_pressure_pa {
        let idx = find_bracketing_index_desc(pressures, origin_p);
        return Ok(ParcelOriginState {
            index: idx,
            theta_override_k: None,
            qv_override_kgkg: None,
            height_override_m: None,
        });
    }
    if let Some(origin_z) = options.origin_height_m {
        let idx = find_bracketing_index_asc(heights, origin_z);
        return Ok(ParcelOriginState {
            index: idx,
            theta_override_k: None,
            qv_override_kgkg: None,
            height_override_m: None,
        });
    }
    match options.cape_type {
        CapeType::SurfaceBased => Ok(ParcelOriginState {
            index: 0,
            theta_override_k: None,
            qv_override_kgkg: None,
            height_override_m: None,
        }),
        CapeType::MixedLayer => {
            let top_p = (pressures[0] - options.mixed_layer_depth_pa.unwrap_or(10000.0))
                .max(*pressures.last().unwrap_or(&pressures[0]));
            let mut indices: Vec<usize> = (0..pressures.len()).filter(|&i| pressures[i] >= top_p).collect();
            if indices.is_empty() {
                indices.push(0);
            }
            let theta_mean = indices
                .iter()
                .map(|&i| potential_temperature(temperatures[i], pressures[i]))
                .sum::<f64>()
                / indices.len() as f64;
            let q_mean = indices.iter().map(|&i| qv[i]).sum::<f64>() / indices.len() as f64;
            Ok(ParcelOriginState {
                index: 0,
                theta_override_k: Some(theta_mean),
                qv_override_kgkg: Some(q_mean),
                height_override_m: Some(heights[0]),
            })
        }
        CapeType::MostUnstable => {
            Ok(ParcelOriginState {
                index: {
                    let min_p = pressures[0] - 30000.0;
                    let mut best_idx = 0usize;
                    let mut best_thetae = f64::NEG_INFINITY;
                    for i in 0..pressures.len() {
                        if pressures[i] < min_p {
                            break;
                        }
                        let td = dewpoint_from_specific_humidity(pressures[i], qv[i]);
                        let thetae = equivalent_potential_temperature(temperatures[i], td, pressures[i]);
                        if thetae > best_thetae {
                            best_thetae = thetae;
                            best_idx = i;
                        }
                    }
                    best_idx
                },
                theta_override_k: None,
                qv_override_kgkg: None,
                height_override_m: None,
            })
        }
        CapeType::UserDefined => Err(EcapeError::OriginNotFound),
    }
}

fn lcl_temperature(temp_k: f64, dewpoint_k: f64) -> f64 {
    1.0 / (1.0 / (dewpoint_k - 56.0) + (temp_k / dewpoint_k).ln() / 800.0) + 56.0
}

fn lcl_pressure(temp_k: f64, dewpoint_k: f64, pressure_pa: f64) -> f64 {
    let tl = lcl_temperature(temp_k, dewpoint_k);
    pressure_pa * (tl / temp_k).powf(1.0 / KAPPA)
}

fn lifting_condensation_level(temp_k: f64, dewpoint_k: f64, pressure_pa: f64) -> (f64, f64) {
    let plcl = lcl_pressure(temp_k, dewpoint_k, pressure_pa);
    let zlcl = (RD * 0.5 * (temp_k + lcl_temperature(temp_k, dewpoint_k)) / G) * (pressure_pa / plcl).ln();
    (plcl, zlcl.max(0.0))
}

fn bunkers_storm_motion(
    pressures: &[f64],
    heights: &[f64],
    u: &[f64],
    v: &[f64],
) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let z0 = heights[0];
    let height_agl: Vec<f64> = heights.iter().map(|z| z - z0).collect();
    let pressure_hpa: Vec<f64> = pressures.iter().map(|p| p / 100.0).collect();
    metrust::calc::bunkers_storm_motion(&pressure_hpa, u, v, &height_agl)
}

fn resolve_storm_motion(
    pressures: &[f64],
    heights: &[f64],
    u: &[f64],
    v: &[f64],
    options: &ParcelOptions,
) -> (f64, f64) {
    if let (Some(u_sm), Some(v_sm)) = (options.storm_motion_u_ms, options.storm_motion_v_ms) {
        return (u_sm, v_sm);
    }
    let z0 = heights[0];
    let heights_agl: Vec<f64> = heights.iter().map(|z| z - z0).collect();
    let (rm, lm, mean) = bunkers_storm_motion(pressures, &heights_agl, u, v);
    match options.storm_motion_type {
        StormMotionType::RightMoving => rm,
        StormMotionType::LeftMoving => lm,
        StormMotionType::MeanWind => mean,
        StormMotionType::UserDefined => options
            .storm_motion_u_ms
            .zip(options.storm_motion_v_ms)
            .unwrap_or(mean),
    }
}

fn calc_sr_wind(
    heights: &[f64],
    u: &[f64],
    v: &[f64],
    storm_u: f64,
    storm_v: f64,
    bottom: f64,
    top: f64,
) -> f64 {
    let z0 = heights[0];
    let mut values = Vec::new();
    for i in 0..heights.len() {
        let agl = heights[i] - z0;
        if agl >= bottom && agl <= top {
            values.push(((u[i] - storm_u).powi(2) + (v[i] - storm_v).powi(2)).sqrt());
        }
    }
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn parcel_profile_from(
    heights: &[f64],
    pressures: &[f64],
    temperatures: &[f64],
    qv_env: &[f64],
    origin_idx: usize,
    entrainment_rate: f64,
    pseudoadiabatic: bool,
    origin_theta_override: Option<f64>,
    origin_qv_override: Option<f64>,
    origin_height_override: Option<f64>,
) -> ParcelProfile {
    let origin_z = origin_height_override.unwrap_or(heights[origin_idx]);
    let mut parcel_pressure = interp_pressure_to_height(heights, pressures, origin_z);
    let mut parcel_height = origin_z;
    let mut parcel_temperature = origin_theta_override
        .map(|theta| temperature_from_potential_temperature(theta, parcel_pressure))
        .unwrap_or(temperatures[origin_idx]);
    let origin_qv = origin_qv_override.unwrap_or(qv_env[origin_idx]);
    let mut parcel_qv = origin_qv;
    let mut parcel_qt = parcel_qv;
    let prate = if pseudoadiabatic { 1.0 / DEFAULT_STEP_M } else { 0.0 };
    let mut dqt_dz = 0.0;

    let mut out_p = vec![parcel_pressure];
    let mut out_z = vec![parcel_height];
    let mut out_t = vec![parcel_temperature];
    let mut out_qv = vec![parcel_qv];
    let mut out_qt = vec![parcel_qt];

    while parcel_pressure >= pressures[pressures.len() - 1] {
        let env_temperature = interp_profile_at_height(heights, temperatures, parcel_height);
        let parcel_saturation_qv = (1.0 - parcel_qt) * r_sat(parcel_temperature, parcel_pressure, 1);
        if parcel_saturation_qv > parcel_qv {
            parcel_pressure = pressure_at_height(parcel_pressure, DEFAULT_STEP_M, env_temperature);
            parcel_height += DEFAULT_STEP_M;
            let env_temperature = interp_profile_at_height(heights, temperatures, parcel_height);
            let env_qv = interp_profile_at_height(heights, qv_env, parcel_height);
            let d_t_dz = unsaturated_adiabatic_lapse_rate(
                parcel_temperature,
                parcel_qv,
                env_temperature,
                env_qv,
                entrainment_rate,
            );
            let dqv_dz = -entrainment_rate * (parcel_qv - env_qv);
            parcel_temperature += d_t_dz * DEFAULT_STEP_M;
            parcel_qv += dqv_dz * DEFAULT_STEP_M;
            parcel_qt = parcel_qv;
        } else {
            parcel_pressure = pressure_at_height(parcel_pressure, DEFAULT_STEP_M, env_temperature);
            parcel_height += DEFAULT_STEP_M;
            let env_temperature = interp_profile_at_height(heights, temperatures, parcel_height);
            let env_qv = interp_profile_at_height(heights, qv_env, parcel_height);
            let d_t_dz = if pseudoadiabatic {
                saturated_adiabatic_lapse_rate(
                    parcel_temperature,
                    parcel_qt,
                    parcel_pressure,
                    env_temperature,
                    env_qv,
                    entrainment_rate,
                    prate,
                    Some(dqt_dz),
                )
            } else {
                saturated_adiabatic_lapse_rate(
                    parcel_temperature,
                    parcel_qt,
                    parcel_pressure,
                    env_temperature,
                    env_qv,
                    entrainment_rate,
                    prate,
                    None,
                )
            };
            let new_parcel_qv = (1.0 - parcel_qt) * r_sat(parcel_temperature, parcel_pressure, 1);
            if pseudoadiabatic {
                dqt_dz = (new_parcel_qv - parcel_qv) / DEFAULT_STEP_M;
            } else {
                dqt_dz = -entrainment_rate * (parcel_qt - env_qv) - prate * (parcel_qt - parcel_qv);
            }
            parcel_temperature += d_t_dz * DEFAULT_STEP_M;
            parcel_qv = new_parcel_qv;
            if pseudoadiabatic {
                parcel_qt = parcel_qv;
            } else {
                dqt_dz = -entrainment_rate * (parcel_qt - env_qv) - prate * (parcel_qt - parcel_qv);
                parcel_qt += dqt_dz * DEFAULT_STEP_M;
            }
            if parcel_qt < parcel_qv {
                parcel_qv = parcel_qt;
            }
        }

        out_p.push(parcel_pressure);
        out_z.push(parcel_height);
        out_t.push(parcel_temperature);
        out_qv.push(parcel_qv);
        out_qt.push(parcel_qt);

        if out_p.len() > 20000 {
            break;
        }
    }

    let buoyancy: Vec<f64> = out_z
        .iter()
        .enumerate()
        .map(|(i, z)| {
            let env_t = interp_profile_at_height(heights, temperatures, *z);
            let env_q = interp_profile_at_height(heights, qv_env, *z);
            let parcel_t_rho = density_temperature(out_t[i], out_qv[i], out_qt[i]);
            let env_t_rho = density_temperature(env_t, env_q, env_q);
            G * (parcel_t_rho - env_t_rho) / env_t_rho
        })
        .collect();
    ParcelProfile {
        pressure_pa: out_p,
        height_m: out_z,
        temperature_k: out_t,
        qv_kgkg: out_qv,
        qt_kgkg: out_qt,
        buoyancy_ms2: buoyancy,
    }
}

pub fn custom_cape_cin_lfc_el(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    qv_kgkg: &[f64],
    options: &ParcelOptions,
) -> Result<CapeCinLfcEl, EcapeError> {
    let zero_wind = vec![0.0; height_m.len()];
    validate_profile(height_m, pressure_pa, temperature_k, qv_kgkg, &zero_wind, &zero_wind)?;
    let origin = resolve_parcel_origin(height_m, pressure_pa, temperature_k, qv_kgkg, options)?;
    let origin_idx = origin.index;

    let profile = parcel_profile_from(
        height_m,
        pressure_pa,
        temperature_k,
        qv_kgkg,
        origin_idx,
        0.0,
        true,
        origin.theta_override_k,
        origin.qv_override_kgkg,
        origin.height_override_m,
    );

    let env_mse: Vec<f64> = height_m
        .iter()
        .zip(temperature_k.iter())
        .zip(qv_kgkg.iter())
        .map(|((z, t), q)| moist_static_energy(*z, *t, *q))
        .collect();
    let height_min_mse_idx = env_mse
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let height_min_mse = height_m[height_min_mse_idx];
    let mut cape = 0.0;
    let mut cin = 0.0;
    let mut lfc = None;
    let mut el = None;
    for i in (1..profile.height_m.len()).rev() {
        let z0 = profile.height_m[i];
        let dz = profile.height_m[i] - profile.height_m[i - 1];
        let env_t = interp_profile_at_height(height_m, temperature_k, z0);
        let env_q = interp_profile_at_height(height_m, qv_kgkg, z0);
        let env_t_rho = density_temperature(env_t, env_q, env_q);
        let parcel_t_rho = density_temperature(profile.temperature_k[i], profile.qv_kgkg[i], profile.qt_kgkg[i]);
        let buoyancy = G * (parcel_t_rho - env_t_rho) / env_t_rho;
        if buoyancy > 0.0 && el.is_none() {
            el = Some(z0);
        }
        if buoyancy > 0.0 && lfc.is_none() {
            cape += buoyancy * dz;
        }
        if z0 < height_min_mse && buoyancy < 0.0 {
            cin += buoyancy * dz;
            if lfc.is_none() {
                lfc = Some(z0);
            }
        }
    }
    if lfc.is_none() {
        lfc = Some(height_m[0]);
    }

    Ok(CapeCinLfcEl {
        cape_jkg: cape,
        cin_jkg: cin,
        lfc_m: lfc,
        el_m: el,
        origin_index: origin_idx,
        pressure_pa: profile.pressure_pa,
        height_m: profile.height_m,
        parcel_temperature_k: profile.temperature_k,
        buoyancy_ms2: profile.buoyancy_ms2,
    })
}

pub fn summarize_parcel_profile(
    parcel_height_m: &[f64],
    parcel_temperature_k: &[f64],
    parcel_qv_kgkg: &[f64],
    parcel_qt_kgkg: &[f64],
    env_height_m: &[f64],
    env_temperature_k: &[f64],
    env_qv_kgkg: &[f64],
) -> CapeCinLfcEl {
    let env_mse: Vec<f64> = env_height_m
        .iter()
        .zip(env_temperature_k.iter())
        .zip(env_qv_kgkg.iter())
        .map(|((z, t), q)| moist_static_energy(*z, *t, *q))
        .collect();
    let height_min_mse_idx = env_mse
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let height_min_mse = env_height_m[height_min_mse_idx];

    let mut cape = 0.0;
    let mut cin = 0.0;
    let mut lfc = None;
    let mut el = None;
    let mut buoyancy_ms2 = Vec::with_capacity(parcel_height_m.len());

    for i in 0..parcel_height_m.len() {
        let z = parcel_height_m[i];
        let env_t = interp_profile_at_height(env_height_m, env_temperature_k, z);
        let env_q = interp_profile_at_height(env_height_m, env_qv_kgkg, z);
        let env_t_rho = density_temperature(env_t, env_q, env_q);
        let parcel_t_rho = density_temperature(parcel_temperature_k[i], parcel_qv_kgkg[i], parcel_qt_kgkg[i]);
        buoyancy_ms2.push(G * (parcel_t_rho - env_t_rho) / env_t_rho);
    }

    for i in (1..parcel_height_m.len()).rev() {
        let z0 = parcel_height_m[i];
        let dz = parcel_height_m[i] - parcel_height_m[i - 1];
        let buoyancy = buoyancy_ms2[i];
        if buoyancy > 0.0 && el.is_none() {
            el = Some(z0);
        }
        if buoyancy > 0.0 && lfc.is_none() {
            cape += buoyancy * dz;
        }
        if z0 < height_min_mse && buoyancy < 0.0 {
            cin += buoyancy * dz;
            if lfc.is_none() {
                lfc = Some(z0);
            }
        }
    }

    if lfc.is_none() {
        lfc = Some(env_height_m[0]);
    }

    CapeCinLfcEl {
        cape_jkg: cape,
        cin_jkg: cin,
        lfc_m: lfc,
        el_m: el,
        origin_index: 0,
        pressure_pa: Vec::new(),
        height_m: parcel_height_m.to_vec(),
        parcel_temperature_k: parcel_temperature_k.to_vec(),
        buoyancy_ms2,
    }
}

fn calc_psi(el_z: f64) -> f64 {
    let sigma = 1.1;
    let alpha = 0.8;
    let l_mix = 120.0;
    let pr = 1.0 / 3.0;
    let ksq = 0.18;
    (ksq * alpha * alpha * std::f64::consts::PI * std::f64::consts::PI * l_mix)
        / (4.0 * pr * sigma * sigma * el_z.max(1.0))
}

fn compute_ncape_reference(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    qv_kgkg: &[f64],
    lfc_m: f64,
    el_m: f64,
) -> f64 {
    if el_m <= lfc_m {
        return 0.0;
    }
    let mse0: Vec<f64> = temperature_k
        .iter()
        .zip(qv_kgkg.iter())
        .zip(height_m.iter())
        .map(|((t, q), z)| moist_static_energy(*z, *t, *q))
        .collect();
    let qsat: Vec<f64> = temperature_k
        .iter()
        .zip(pressure_pa.iter())
        .map(|(t, p)| {
            let rsat = r_sat(*t, *p, 0);
            rsat / (1.0 + rsat)
        })
        .collect();
    let mse0_star: Vec<f64> = temperature_k
        .iter()
        .zip(qsat.iter())
        .zip(height_m.iter())
        .map(|((t, q), z)| moist_static_energy(*z, *t, *q))
        .collect();
    let mut mse0bar = vec![0.0; mse0.len()];
    mse0bar[0] = mse0[0];
    for iz in 1..mse0bar.len() {
        let mut sum = 0.0;
        for j in 0..iz {
            sum += (mse0[j] + mse0[j + 1]) * (height_m[j + 1] - height_m[j]);
        }
        mse0bar[iz] = 0.5 * sum / (height_m[iz] - height_m[0]);
    }
    let int_arg: Vec<f64> = mse0bar
        .iter()
        .zip(mse0_star.iter())
        .zip(temperature_k.iter())
        .map(|((bar, star), t)| -(G / (CPD * *t)) * (bar - star))
        .collect();
    let ind_lfc = height_m
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - lfc_m).abs().partial_cmp(&(b.1 - lfc_m).abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    let ind_el = height_m
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - el_m).abs().partial_cmp(&(b.1 - el_m).abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(ind_lfc);
    if ind_el <= ind_lfc + 1 {
        return 0.0;
    }
    let mut ncape = 0.0;
    for i in ind_lfc..(ind_el - 1) {
        ncape += (0.5 * int_arg[i] + 0.5 * int_arg[i + 1]) * (height_m[i + 1] - height_m[i]);
    }
    ncape.max(0.0)
}

fn calc_ecape_a(sr_wind: f64, psi: f64, ncape: f64, cape: f64) -> f64 {
    let sr2 = (sr_wind * sr_wind).max(1e-9);
    let denom = 4.0 * psi / sr2;
    let term_a = sr2 / 2.0;
    let term_b = (-1.0 - psi - (2.0 * psi / sr2) * ncape) / denom;
    let term_c = ((1.0 + psi + (2.0 * psi / sr2) * ncape).powi(2) + 8.0 * (psi / sr2) * (cape - psi * ncape)).sqrt()
        / denom;
    let ecape_a = term_a + term_b + term_c;
    if ecape_a >= 0.0 { ecape_a } else { 0.0 }
}

fn entrainment_rate(cape: f64, ecape: f64, ncape: f64, vsr: f64, storm_column_height: f64) -> f64 {
    let e_a_tilde = ecape / cape.max(1e-9);
    let n_tilde = ncape / cape.max(1e-9);
    let vsr_tilde = vsr / (2.0 * cape.max(1e-9)).sqrt();
    let e_tilde = e_a_tilde - vsr_tilde * vsr_tilde;
    (2.0 * (1.0 - e_tilde) / (e_tilde + n_tilde)) / storm_column_height.max(1e-9)
}

pub fn calc_ecape_ncape_from_reference(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    qv_kgkg: &[f64],
    u_wind_ms: &[f64],
    v_wind_ms: &[f64],
    options: &ParcelOptions,
    cape: f64,
    lfc_m: Option<f64>,
    el_m: Option<f64>,
) -> EcapeNcape {
    let (storm_u, storm_v) = resolve_storm_motion(pressure_pa, height_m, u_wind_ms, v_wind_ms, options);
    let bottom = options.inflow_layer_bottom_m.unwrap_or(0.0);
    let top = options.inflow_layer_top_m.unwrap_or(1000.0);
    let vsr = calc_sr_wind(height_m, u_wind_ms, v_wind_ms, storm_u, storm_v, bottom, top);
    let ncape = match (lfc_m, el_m) {
        (Some(lfc), Some(el)) if el > lfc => {
            compute_ncape_reference(height_m, pressure_pa, temperature_k, qv_kgkg, lfc, el)
        }
        _ => 0.0,
    };
    let psi = el_m.map(calc_psi).unwrap_or(0.0);
    let ecape = if el_m.is_some() && psi > 0.0 && vsr > 0.0 {
        calc_ecape_a(vsr, psi, ncape, cape)
    } else {
        0.0
    };
    EcapeNcape {
        ecape_jkg: ecape,
        ncape_jkg: ncape,
        cape_jkg: cape,
        lfc_m,
        el_m,
        storm_motion_u_ms: storm_u,
        storm_motion_v_ms: storm_v,
        storm_relative_wind_ms: vsr,
        psi,
    }
}

pub fn calc_ecape_ncape(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    qv_kgkg: &[f64],
    u_wind_ms: &[f64],
    v_wind_ms: &[f64],
    options: &ParcelOptions,
) -> Result<EcapeNcape, EcapeError> {
    validate_profile(height_m, pressure_pa, temperature_k, qv_kgkg, u_wind_ms, v_wind_ms)?;
    let cape_info = custom_cape_cin_lfc_el(height_m, pressure_pa, temperature_k, qv_kgkg, options)?;
    Ok(calc_ecape_ncape_from_reference(
        height_m,
        pressure_pa,
        temperature_k,
        qv_kgkg,
        u_wind_ms,
        v_wind_ms,
        options,
        cape_info.cape_jkg,
        cape_info.lfc_m,
        cape_info.el_m,
    ))
}

pub fn calc_ecape_parcel(
    height_m: &[f64],
    pressure_pa: &[f64],
    temperature_k: &[f64],
    dewpoint_k: &[f64],
    u_wind_ms: &[f64],
    v_wind_ms: &[f64],
    options: &ParcelOptions,
) -> Result<EcapeParcelResult, EcapeError> {
    let qv: Vec<f64> = pressure_pa
        .iter()
        .zip(dewpoint_k.iter())
        .map(|(p, td)| specific_humidity_from_dewpoint(*p, *td))
        .collect();
    validate_profile(height_m, pressure_pa, temperature_k, &qv, u_wind_ms, v_wind_ms)?;
    let origin = resolve_parcel_origin(height_m, pressure_pa, temperature_k, &qv, options)?;
    let origin_idx = origin.index;
    let origin_z = origin.height_override_m.unwrap_or(height_m[origin_idx]);
    let pseudoadiabatic = options.pseudoadiabatic.unwrap_or(true);
    let base_profile = parcel_profile_from(
        height_m,
        pressure_pa,
        temperature_k,
        &qv,
        origin_idx,
        0.0,
        pseudoadiabatic,
        origin.theta_override_k,
        origin.qv_override_kgkg,
        origin.height_override_m,
    );
    let base = summarize_parcel_profile(
        &base_profile.height_m,
        &base_profile.temperature_k,
        &base_profile.qv_kgkg,
        &base_profile.qt_kgkg,
        height_m,
        temperature_k,
        &qv,
    );
    let ecape_info = calc_ecape_ncape_from_reference(
        height_m,
        pressure_pa,
        temperature_k,
        &qv,
        u_wind_ms,
        v_wind_ms,
        options,
        base.cape_jkg,
        base.lfc_m,
        base.el_m,
    );
    let entrainment = options.entrainment_rate.unwrap_or_else(|| {
        if let (Some(el), vsr) = (ecape_info.el_m, ecape_info.storm_relative_wind_ms) {
            if el > origin_z && base.cape_jkg > 0.0 {
                entrainment_rate(base.cape_jkg, ecape_info.ecape_jkg, ecape_info.ncape_jkg, vsr, el - origin_z)
            } else {
                0.0
            }
        } else {
            0.0
        }
    });

    let parcel = parcel_profile_from(
        height_m,
        pressure_pa,
        temperature_k,
        &qv,
        origin_idx,
        entrainment.max(0.0),
        pseudoadiabatic,
        origin.theta_override_k,
        origin.qv_override_kgkg,
        origin.height_override_m,
    );

    let parcel_summary = summarize_parcel_profile(
        &parcel.height_m,
        &parcel.temperature_k,
        &parcel.qv_kgkg,
        &parcel.qt_kgkg,
        height_m,
        temperature_k,
        &qv,
    );
    let parcel_ecape = calc_ecape_ncape_from_reference(
        height_m,
        pressure_pa,
        temperature_k,
        &qv,
        u_wind_ms,
        v_wind_ms,
        options,
        parcel_summary.cape_jkg,
        parcel_summary.lfc_m,
        parcel_summary.el_m,
    );

    Ok(EcapeParcelResult {
        ecape_jkg: parcel_ecape.ecape_jkg,
        ncape_jkg: parcel_ecape.ncape_jkg,
        cape_jkg: parcel_summary.cape_jkg,
        cin_jkg: parcel_summary.cin_jkg,
        lfc_m: parcel_summary.lfc_m,
        el_m: parcel_summary.el_m,
        storm_motion_u_ms: parcel_ecape.storm_motion_u_ms,
        storm_motion_v_ms: parcel_ecape.storm_motion_v_ms,
        parcel_profile: parcel,
    })
}
