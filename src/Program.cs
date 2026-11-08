using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Windows.Forms;

namespace HabboSiClient
{
    internal static class Program
    {
        /// <summary>Set by --shot: render the chrome to a PNG instead of starting the hotel.</summary>
        public static string ShotPath;
        /// <summary>Set by --smoke: navigate for real, write a result log and exit.</summary>
        public static string SmokeLog;
        public static string SmokeImage;
        /// <summary>Set by --shot-live: capture the window with the hotel loaded inside it.</summary>
        public static string ShotLive;
        /// <summary>Set by --probe: dump room/DOM diagnostics from the loaded client and exit.</summary>
        public static string ProbePath;

        [STAThread]
        private static void Main(string[] args)
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            try { SetProcessDPIAware(); }
            catch { }

            float dpi = 96f;
            try
            {
                using (Graphics g = Graphics.FromHwnd(IntPtr.Zero)) dpi = g.DpiX;
            }
            catch { }
            if (dpi < 96f) dpi = 96f;
            T.S = dpi / 96f;

            string forced = Value(args, "--scale");
            bool forcedScale = false;
            if (forced != null)
            {
                float f;
                if (float.TryParse(forced, System.Globalization.NumberStyles.Float,
                        System.Globalization.CultureInfo.InvariantCulture, out f) && f >= 1f && f <= 3f)
                {
                    T.S = f;
                    forcedScale = true;
                }
            }
            T.Init(forcedScale ? T.S : 1f);
            Store.Ensure();

            ClientSettings settings = Store.Load();

            string lang = Value(args, "--lang");
            if (lang != null) L.De = lang.Equals("de", StringComparison.OrdinalIgnoreCase);
            else L.De = settings.German;

            string theme = Value(args, "--theme");
            if (theme != null)
            {
                if (theme.Equals("light", StringComparison.OrdinalIgnoreCase)) settings.Mode = ThemeMode.Light;
                else if (theme.Equals("navy", StringComparison.OrdinalIgnoreCase)) settings.Mode = ThemeMode.Navy;
                else if (theme.Equals("custom", StringComparison.OrdinalIgnoreCase)) settings.Mode = ThemeMode.Custom;
                else settings.Mode = ThemeMode.Dark;
            }
            T.Apply(settings);

            // developer aid: write the exact SET_ACTIVITY payload without touching Discord
            string presenceJson = Value(args, "--presence-json");
            if (presenceJson != null)
            {
                File.WriteAllText(presenceJson, DiscordRpc.BuildActivityJson(
                    Process.GetCurrentProcess().Id,
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    ClientForm.PresenceDetails("SUNSET LOUNGE"), ClientForm.PresenceState,
                    DiscordRpc.ArtworkUrl,
                    DiscordRpc.LargeText));
                return;
            }

            // developer aid: take the real GitHub release lookup without starting the UI
            string updatePath = Value(args, "--update-check");
            if (updatePath != null)
            {
                UpdateInfo info = UpdateCheck.Latest();
                string text =
                    "running=" + AppInfo.Version + "\r\n" +
                    "latest=" + (info != null ? info.Latest : "") + "\r\n" +
                    "page=" + (info != null ? info.PageUrl : "") + "\r\n" +
                    "newer=" + (info != null && UpdateCheck.IsNewer(info.Latest)) + "\r\n" +
                    "error=" + (info == null ? "no release or network" : "") + "\r\n";
                File.WriteAllText(updatePath, text);
                return;
            }

            ShotPath = Value(args, "--shot");
            SmokeLog = Value(args, "--smoke");
            SmokeImage = Value(args, "--smoke-shot");
            ShotLive = Value(args, "--shot-live");
            ProbePath = Value(args, "--probe");
            string shotSettings = Value(args, "--shot-settings");
            if (shotSettings != null)
            {
                ShotSheet(settings, shotSettings);
                return;
            }

            bool noSplash = Has(args, "--nosplash") || ShotPath != null || SmokeLog != null || ShotLive != null || ProbePath != null;
            if (!noSplash) Application.Run(new SplashForm());

            if (ShotPath != null)
            {
                Shot(settings);
                return;
            }

            Application.Run(new ClientForm(settings));
        }

        /// <summary>Renders the settings sheet to a PNG for review.</summary>
        private static void ShotSheet(ClientSettings settings, string path)
        {
            using (SettingsSheet f = new SettingsSheet(settings, null))
            {
                f.StartPosition = FormStartPosition.Manual;
                f.Location = new Point(-20000, -20000);
                f.Show();
                Application.DoEvents();
                f.Refresh();
                using (Bitmap bmp = new Bitmap(f.Width, f.Height))
                {
                    f.DrawToBitmap(bmp, new Rectangle(0, 0, f.Width, f.Height));
                    bmp.Save(path, ImageFormat.Png);
                }
                f.Close();
            }
        }

        /// <summary>Renders the chrome (without the WebView2 surface) to a PNG for review.</summary>
        private static void Shot(ClientSettings settings)
        {
            using (ClientForm f = new ClientForm(settings))
            {
                f.StartPosition = FormStartPosition.Manual;
                f.Location = new Point(-20000, -20000);
                f.Show();
                Application.DoEvents();
                f.Refresh();
                using (Bitmap bmp = new Bitmap(f.Width, f.Height))
                {
                    f.DrawToBitmap(bmp, new Rectangle(0, 0, f.Width, f.Height));
                    bmp.Save(ShotPath, ImageFormat.Png);
                }
                f.Close();
            }
        }

        private static string Value(string[] args, string name)
        {
            for (int i = 0; i < args.Length; i++)
            {
                if (!string.Equals(args[i], name, StringComparison.OrdinalIgnoreCase)) continue;
                if (i + 1 >= args.Length) return null;
                string v = args[i + 1];
                if (v.StartsWith("--", StringComparison.Ordinal)) return null;
                return v;
            }
            return null;
        }

        private static bool Has(string[] args, string name)
        {
            for (int i = 0; i < args.Length; i++)
                if (string.Equals(args[i], name, StringComparison.OrdinalIgnoreCase)) return true;
            return false;
        }

        [System.Runtime.InteropServices.DllImport("user32.dll")]
        private static extern bool SetProcessDPIAware();
    }
}
