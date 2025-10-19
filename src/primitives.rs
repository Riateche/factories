use {
    crate::rf,
    derive_more::{From, Into},
    once_cell::sync::Lazy,
    ordered_float::OrderedFloat,
    serde::{de::Error, Deserialize, Serialize},
    std::{
        fmt::{self, Display, Formatter},
        iter::Sum,
        ops::{Add, Mul, Neg, Sub, SubAssign},
        str::FromStr,
    },
    tracing::error,
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct Speed(OrderedFloat<f64>);

impl Speed {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
    pub const ONE: Self = Self(OrderedFloat(1.0));
}

impl From<f64> for Speed {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<Speed> for f64 {
    fn from(value: Speed) -> Self {
        value.0.into()
    }
}

impl FromStr for Speed {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(f64::from_str(s)?.into())
    }
}

impl Display for Speed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/s", rf((*self).into()))
    }
}

impl Neg for Speed {
    type Output = Speed;

    fn neg(self) -> Self::Output {
        (-f64::from(self)).into()
    }
}

impl Sum<Speed> for Speed {
    fn sum<I: Iterator<Item = Speed>>(iter: I) -> Self {
        iter.map(f64::from).sum::<f64>().into()
    }
}

impl SubAssign for Speed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 .0 -= rhs.0 .0;
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct MachineCount(OrderedFloat<f64>);

impl From<f64> for MachineCount {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<MachineCount> for f64 {
    fn from(value: MachineCount) -> Self {
        value.0.into()
    }
}

impl FromStr for MachineCount {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(f64::from_str(s)?.into())
    }
}

impl Display for MachineCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", rf((*self).into()))
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    From,
    Into,
    Serialize,
    Deserialize,
)]
pub struct Amount(OrderedFloat<f64>);

impl Amount {
    pub const ZERO: Self = Self(OrderedFloat(0.0));
    pub const ONE: Self = Self(OrderedFloat(1.0));
}

impl From<f64> for Amount {
    fn from(value: f64) -> Self {
        Self(value.into())
    }
}

impl From<Amount> for f64 {
    fn from(value: Amount) -> Self {
        value.0.into()
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", rf((*self).into()))
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Self) -> Self::Output {
        (f64::from(self) + f64::from(rhs)).into()
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        (f64::from(self) - f64::from(rhs)).into()
    }
}

impl Mul<Speed> for Amount {
    type Output = Speed;

    fn mul(self, rhs: Speed) -> Self::Output {
        (f64::from(self) * f64::from(rhs)).into()
    }
}

impl Mul<Amount> for Speed {
    type Output = Speed;

    fn mul(self, rhs: Amount) -> Self::Output {
        (f64::from(self) * f64::from(rhs)).into()
    }
}

impl Mul<f64> for Speed {
    type Output = Speed;

    fn mul(self, rhs: f64) -> Self::Output {
        (f64::from(self) * rhs).into()
    }
}

impl Mul<Speed> for f64 {
    type Output = Speed;

    fn mul(self, rhs: Speed) -> Self::Output {
        (self * f64::from(rhs)).into()
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct ItemName(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct RecipeName(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct CrafterName(pub String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct ModuleName(String);

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct RecipeCategory(pub String);

impl Display for ItemName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for RecipeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for CrafterName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for ModuleName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Display for RecipeCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialEq<&str> for ItemName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
impl PartialEq<&str> for RecipeCategory {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl ItemName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl ModuleName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl CrafterName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl RecipeName {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl From<&str> for ItemName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for ModuleName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for CrafterName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for RecipeName {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}
impl From<&str> for RecipeCategory {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

pub static SOURCE_CRAFTER_NAME: Lazy<CrafterName> = Lazy::new(|| "source".into());
pub static SINK_CRAFTER_NAME: Lazy<CrafterName> = Lazy::new(|| "sink".into());
pub static SOURCE_RECIPE_CATEGORY: Lazy<RecipeCategory> = Lazy::new(|| "source".into());
pub static SINK_RECIPE_CATEGORY: Lazy<RecipeCategory> = Lazy::new(|| "sink".into());

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    From,
    Into,
    Serialize,
    Deserialize,
)]
pub struct Quality(pub u32);

impl Quality {
    pub fn as_f64(self) -> f64 {
        self.0.into()
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub fn next(self) -> Option<Quality> {
        match self.0 {
            0..=2 => Some(Self(self.0 + 1)),
            3 => Some(Self(5)),
            5 => None,
            _ => {
                error!("invalid quality: {:?}", self);
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into)]
pub struct ItemNameAndQuality {
    pub name: ItemName,
    pub quality: Quality,
}

impl<'de> Deserialize<'de> for ItemNameAndQuality {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

impl Display for ItemNameAndQuality {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.quality.0 > 0 {
            write!(f, "{}.q{}", self.name, self.quality.0)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

impl FromStr for ItemNameAndQuality {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((name, quality)) = s.split_once(".q") {
            Ok(Self {
                name: name.into(),
                quality: Quality(quality.parse()?),
            })
        } else {
            Ok(Self {
                name: s.into(),
                quality: Quality::default(),
            })
        }
    }
}

impl Serialize for ItemNameAndQuality {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}
