// The `#[sabi_trait]` macro in abi_stable 0.11 emits an `impl` the
// `non_local_definitions` lint flags; it originates in the macro, not our code.
#![allow(non_local_definitions)]

//! Stable ABI contract between the launcher host and dynamically-loaded
//! plugins.
//!
//! Both the host and every plugin depend on this crate at a compatible
//! version. Plugins are compiled as `cdylib`s and loaded at runtime; the
//! [`abi_stable`] layer verifies that the plugin was built against a compatible
//! layout before any call is made, so an out-of-date plugin fails to load
//! rather than crashing the host.
//!
//! ## Writing a plugin
//!
//! ```ignore
//! use plugin_api::{export_plugin, AbiPluginResult, HostPlugin};
//! use abi_stable::std_types::{RStr, RString, RVec};
//!
//! #[derive(Default)]
//! struct MyPlugin;
//!
//! impl HostPlugin for MyPlugin {
//!     fn id(&self) -> RString { "my_plugin".into() }
//!     fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult> {
//!         // inspect `query`, return results (or an empty RVec)
//!         RVec::new()
//!     }
//! }
//!
//! export_plugin!(MyPlugin);
//! ```

use abi_stable::{
    StableAbi,
    library::RootModule,
    package_version_strings,
    sabi_trait,
    sabi_types::VersionStrings,
    std_types::{RBox, ROption, RStr, RString, RVec},
};

pub use abi_stable;
pub use abi_stable::sabi_trait::TD_Opaque;
pub use abi_stable::std_types;

/// A result surfaced by a plugin for the current query. FFI-safe mirror of the
/// host's internal result type.
#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub struct AbiPluginResult {
    /// Id of the plugin that produced this result.
    pub source_id: RString,
    /// Section header this result groups under (e.g. "Calculator").
    pub section: RString,
    /// Primary line.
    pub title: RString,
    /// Optional secondary line.
    pub subtitle: ROption<RString>,
    /// Optional path to an icon file on disk.
    pub icon_path: ROption<RString>,
    /// Optional single-character glyph for the fallback tile, as a `char`
    /// scalar value.
    pub glyph: ROption<u32>,
    /// Actions, most important first (the first is the default action).
    pub actions: RVec<AbiPluginAction>,
}

/// One entry in a result's action list.
#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub struct AbiPluginAction {
    pub label: RString,
    pub effect: AbiActionEffect,
}

/// A side effect the host performs when an action is invoked.
#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub enum AbiActionEffect {
    /// Copy the given text to the system clipboard.
    CopyToClipboard(RString),
}

/// The interface a plugin implements. Object-safe and FFI-safe via
/// [`abi_stable`]'s `sabi_trait`, producing the `HostPlugin_TO` trait object.
#[sabi_trait]
pub trait HostPlugin: Send + Sync {
    /// Stable identifier for the plugin.
    fn id(&self) -> RString;

    /// Results for `query`. Return an empty `RVec` when the query doesn't apply.
    fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult>;
}

/// The root module a plugin library exports. Its single entry point constructs
/// the plugin's trait object.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = PluginModRef)))]
#[sabi(missing_field(panic))]
pub struct PluginMod {
    /// Construct a fresh plugin instance.
    #[sabi(last_prefix_field)]
    pub new: extern "C" fn() -> HostPlugin_TO<'static, RBox<()>>,
}

impl RootModule for PluginModRef {
    abi_stable::declare_root_module_statics! {PluginModRef}
    const BASE_NAME: &'static str = "iced_raycast_plugin";
    const NAME: &'static str = "iced_raycast_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}

/// Wrap a plugin value into the FFI-safe trait object the host consumes.
pub fn plugin_object<P>(plugin: P) -> HostPlugin_TO<'static, RBox<()>>
where
    P: HostPlugin + 'static,
{
    HostPlugin_TO::from_value(plugin, TD_Opaque)
}

/// Generate the `#[export_root_module]` boilerplate for a plugin type that
/// implements [`HostPlugin`] and [`Default`].
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        #[$crate::abi_stable::export_root_module]
        fn __instantiate_plugin_root_module() -> $crate::PluginModRef {
            use $crate::abi_stable::prefix_type::PrefixTypeTrait;
            $crate::PluginMod {
                new: __new_plugin_instance,
            }
            .leak_into_prefix()
        }

        #[$crate::abi_stable::sabi_extern_fn]
        fn __new_plugin_instance()
        -> $crate::HostPlugin_TO<'static, $crate::abi_stable::std_types::RBox<()>> {
            $crate::plugin_object(<$plugin as ::core::default::Default>::default())
        }
    };
}
