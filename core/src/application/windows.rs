use crate::{Application, Image};
use std::ffi::c_void;
use std::process::Command;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppRGBA,
    IWICImagingFactory, WICBitmapUseAlpha, WICRect,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize,
};
use windows::Win32::UI::Shell::{
    BHID_EnumItems, IEnumShellItems, IShellItem, IShellItemImageFactory,
    SHCreateItemFromParsingName, SIGDN, SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
    SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY,
};
use windows::core::{Interface, w};

/// Size, in pixels, of the icons we request from the shell for each app.
const ICON_SIZE: i32 = 32;

#[derive(Clone, Debug)]
pub struct WindowsApplication {
    pub name: String,
    /// The app's parsing name relative to the shell `AppsFolder` — an
    /// AppUserModelID for Store/UWP apps (`…_8wekyb3d8bbwe!App`) or a link
    /// identifier for classic apps. Launched via `shell:AppsFolder\<app_id>`.
    pub app_id: String,
    pub icon: Option<Image>,
}

/// Restores the COM reference count taken by our `CoInitializeEx` when the
/// enumeration finishes.
struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

impl Application for WindowsApplication {
    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        // The shell `AppsFolder` is the virtual folder backing Start → "All
        // apps": it lists both classic Win32 programs and Store/UWP apps, the
        // curated set a user actually launches — unlike the registry's
        // `…\Uninstall` keys, which also contain redistributables, SDKs, driver
        // packages and KB updates, and so produced an unusably long list.
        unsafe { enumerate_apps_folder() }.unwrap_or_default()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        None
    }

    fn icon(&self) -> Option<Image> {
        self.icon.clone()
    }

    fn execute(&self, _arg: Option<String>) -> anyhow::Result<()> {
        // Handing the `shell:AppsFolder\<id>` moniker to Explorer lets the shell
        // activate the app the same way the Start menu does — this is the only
        // launch path that works for both Win32 and Store/UWP apps.
        Command::new("explorer.exe")
            .arg(format!("shell:AppsFolder\\{}", self.app_id))
            .spawn()?;

        Ok(())
    }
}

/// Enumerates every entry in the shell `AppsFolder`, resolving each one's
/// display name, launch id and icon.
unsafe fn enumerate_apps_folder() -> windows::core::Result<Vec<WindowsApplication>> {
    // Only balance the ref count when *we* initialized COM on this thread; if
    // the caller already did (returning S_FALSE / RPC_E_CHANGED_MODE), leave it
    // to them.
    let _com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
        .is_ok()
        .then_some(ComGuard);

    let apps_folder: IShellItem =
        unsafe { SHCreateItemFromParsingName(w!("shell:AppsFolder"), None) }?;
    let items: IEnumShellItems = unsafe { apps_folder.BindToHandler(None, &BHID_EnumItems) }?;

    // A single WIC factory, reused to turn each shell icon into RGBA pixels.
    let imaging: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL) }?;

    let mut apps = Vec::new();
    loop {
        let mut buffer: [Option<IShellItem>; 1] = [None];
        let mut fetched = 0u32;

        if unsafe { items.Next(&mut buffer, Some(&mut fetched)) }.is_err() || fetched == 0 {
            break;
        }

        let Some(item) = buffer[0].take() else {
            break;
        };

        let names = unsafe {
            (
                display_name(&item, SIGDN_NORMALDISPLAY),
                display_name(&item, SIGDN_PARENTRELATIVEPARSING),
            )
        };
        let (Some(name), Some(app_id)) = names else {
            continue;
        };

        let icon = item
            .cast::<IShellItemImageFactory>()
            .ok()
            .and_then(|factory| unsafe { icon_pixels(&factory, &imaging) });

        apps.push(WindowsApplication { name, app_id, icon });
    }

    Ok(apps)
}

/// Reads one of a shell item's display names, freeing the shell-allocated
/// string afterwards.
unsafe fn display_name(item: &IShellItem, kind: SIGDN) -> Option<String> {
    let pwstr = unsafe { item.GetDisplayName(kind) }.ok()?;
    let value = unsafe { pwstr.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(pwstr.0 as *const c_void)) };
    value.filter(|s| !s.is_empty())
}

/// Renders a shell item's icon into an RGBA image via WIC.
unsafe fn icon_pixels(
    factory: &IShellItemImageFactory,
    imaging: &IWICImagingFactory,
) -> Option<Image> {
    let size = SIZE {
        cx: ICON_SIZE,
        cy: ICON_SIZE,
    };
    let bitmap = unsafe { factory.GetImage(size, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK) }.ok()?;

    let image = (|| {
        let wic =
            unsafe { imaging.CreateBitmapFromHBITMAP(bitmap, None, WICBitmapUseAlpha) }.ok()?;

        let format = unsafe { wic.GetPixelFormat() }.ok()?;
        if format != GUID_WICPixelFormat32bppBGRA && format != GUID_WICPixelFormat32bppRGBA {
            return None;
        }

        let rect = WICRect {
            X: 0,
            Y: 0,
            Width: ICON_SIZE,
            Height: ICON_SIZE,
        };
        let mut pixels = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
        unsafe { wic.CopyPixels(&rect, ICON_SIZE as u32 * 4, &mut pixels) }.ok()?;

        // The shell hands us BGRA; the UI expects RGBA.
        if format == GUID_WICPixelFormat32bppBGRA {
            for chunk in pixels.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }
        }

        Some(Image::Rgba(ICON_SIZE as u32, ICON_SIZE as u32, pixels))
    })();

    let _ = unsafe { DeleteObject(bitmap) };
    image
}
