using System;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;

namespace HabboSiClient
{
    /// <summary>Everything the client remembers between runs.</summary>
    internal class ClientSettings
    {
        public ThemeMode Mode = ThemeMode.Dark;
        public bool German;
        public float Zoom = 1f;
        public bool Gpu = true;
        public bool NoThrottle = true;
        public bool ShowFps = true;
        public bool DevTools;
        public bool Discord = true;
        public string DiscordAppId = "";
        public int Width = 1360, Height = 860;
        public bool Maximized;

        // kept so the custom palette stays loadable; the client offers the built-in themes
        public Color SurfaceColor = Color.FromArgb(18, 32, 58);
        public Color AccentColor = Color.FromArgb(246, 196, 82);
        public Color TextColor = Color.FromArgb(230, 239, 252);

        public ClientSettings Clone()
        {
            ClientSettings c = new ClientSettings();
            c.Mode = Mode;
            c.German = German;
            c.Zoom = Zoom;
            c.Gpu = Gpu;
            c.NoThrottle = NoThrottle;
            c.ShowFps = ShowFps;
            c.DevTools = DevTools;
            c.Discord = Discord;
            c.DiscordAppId = DiscordAppId;
            c.Width = Width;
            c.Height = Height;
            c.Maximized = Maximized;
            c.SurfaceColor = SurfaceColor;
            c.AccentColor = AccentColor;
            c.TextColor = TextColor;
            return c;
        }
    }

    /// <summary>
    /// The client's own data folder. Only two files live here: settings.json and the
    /// WebView2 profile that keeps the hotel login and the asset cache warm between runs.
    /// </summary>
    internal static class Store
    {
        public static string Dir
        {
            get
            {
                string root = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                return Path.Combine(root, "HabboSiClient");
            }
        }

        public static string ProfileDir { get { return Path.Combine(Dir, "WebView2"); } }
        public static string SettingsPath { get { return Path.Combine(Dir, "settings.json"); } }

        public static void Ensure()
        {
            try
            {
                Directory.CreateDirectory(Dir);
                Directory.CreateDirectory(ProfileDir);
            }
            catch { }
        }

        public static ClientSettings Load()
        {
            ClientSettings s = new ClientSettings();
            try
            {
                if (!File.Exists(SettingsPath)) return s;
                string json = File.ReadAllText(SettingsPath, Encoding.UTF8);

                string mode = Str(json, "theme");
                if (mode == "light") s.Mode = ThemeMode.Light;
                else if (mode == "navy") s.Mode = ThemeMode.Navy;
                else if (mode == "custom") s.Mode = ThemeMode.Custom;
                else s.Mode = ThemeMode.Dark;

                s.German = Bool(json, "german", s.German);
                s.Zoom = (float)Clamp(Num(json, "zoom", s.Zoom), 0.5, 2.0);
                s.Gpu = Bool(json, "gpu", s.Gpu);
                s.NoThrottle = Bool(json, "nothrottle", s.NoThrottle);
                s.ShowFps = Bool(json, "showfps", s.ShowFps);
                s.DevTools = Bool(json, "devtools", s.DevTools);
                s.Discord = Bool(json, "discord", s.Discord);
                string discordId = Str(json, "discordid");
                if (discordId != null) s.DiscordAppId = discordId;
                s.Width = (int)Clamp(Num(json, "width", s.Width), 900, 10000);
                s.Height = (int)Clamp(Num(json, "height", s.Height), 600, 10000);
                s.Maximized = Bool(json, "maximized", s.Maximized);

                Color c;
                if (ColorFromHex(Str(json, "surface"), out c)) s.SurfaceColor = c;
                if (ColorFromHex(Str(json, "accent"), out c)) s.AccentColor = c;
                if (ColorFromHex(Str(json, "text"), out c)) s.TextColor = c;
            }
            catch { }
            return s;
        }

        public static void Save(ClientSettings s)
        {
            try
            {
                Directory.CreateDirectory(Dir);
                StringBuilder b = new StringBuilder();
                b.AppendLine("{");
                b.AppendLine("  \"theme\": \"" + ModeName(s.Mode) + "\",");
                b.AppendLine("  \"german\": " + (s.German ? "true" : "false") + ",");
                b.AppendLine("  \"zoom\": " + s.Zoom.ToString("0.00", CultureInfo.InvariantCulture) + ",");
                b.AppendLine("  \"gpu\": " + (s.Gpu ? "true" : "false") + ",");
                b.AppendLine("  \"nothrottle\": " + (s.NoThrottle ? "true" : "false") + ",");
                b.AppendLine("  \"showfps\": " + (s.ShowFps ? "true" : "false") + ",");
                b.AppendLine("  \"devtools\": " + (s.DevTools ? "true" : "false") + ",");
                b.AppendLine("  \"discord\": " + (s.Discord ? "true" : "false") + ",");
                b.AppendLine("  \"discordid\": \"" + Escape(s.DiscordAppId) + "\",");
                b.AppendLine("  \"width\": " + s.Width + ",");
                b.AppendLine("  \"height\": " + s.Height + ",");
                b.AppendLine("  \"maximized\": " + (s.Maximized ? "true" : "false") + ",");
                b.AppendLine("  \"surface\": \"" + Hex(s.SurfaceColor) + "\",");
                b.AppendLine("  \"accent\": \"" + Hex(s.AccentColor) + "\",");
                b.AppendLine("  \"text\": \"" + Hex(s.TextColor) + "\"");
                b.AppendLine("}");
                File.WriteAllText(SettingsPath, b.ToString(), Encoding.UTF8);
            }
            catch { }
        }

        static string ModeName(ThemeMode m)
        {
            if (m == ThemeMode.Light) return "light";
            if (m == ThemeMode.Navy) return "navy";
            if (m == ThemeMode.Custom) return "custom";
            return "dark";
        }

        static string Escape(string s)
        {
            if (string.IsNullOrEmpty(s)) return "";
            return s.Replace("\\", "\\\\").Replace("\"", "\\\"");
        }

        static string Hex(Color c)
        {
            return "#" + c.R.ToString("X2") + c.G.ToString("X2") + c.B.ToString("X2");
        }

        static bool ColorFromHex(string text, out Color c)
        {
            c = Color.Empty;
            if (text == null) return false;
            text = text.Trim().TrimStart('#');
            if (text.Length != 6) return false;
            int v;
            if (!int.TryParse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out v)) return false;
            c = Color.FromArgb((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF);
            return true;
        }

        static double Clamp(double v, double lo, double hi)
        {
            if (v < lo) return lo;
            if (v > hi) return hi;
            return v;
        }

        static Match Find(string json, string key)
        {
            return Regex.Match(json, "\"" + Regex.Escape(key) + "\"\\s*:\\s*(\"[^\"]*\"|true|false|-?[0-9.]+)",
                RegexOptions.IgnoreCase);
        }

        static string Str(string json, string key)
        {
            Match m = Find(json, key);
            if (!m.Success) return null;
            string raw = m.Groups[1].Value;
            if (raw.Length >= 2 && raw[0] == '"' && raw[raw.Length - 1] == '"')
                return raw.Substring(1, raw.Length - 2);
            return raw;
        }

        static bool Bool(string json, string key, bool fallback)
        {
            Match m = Find(json, key);
            if (!m.Success) return fallback;
            return m.Groups[1].Value == "true";
        }

        static double Num(string json, string key, double fallback)
        {
            Match m = Find(json, key);
            if (!m.Success) return fallback;
            double v;
            if (double.TryParse(m.Groups[1].Value, NumberStyles.Float, CultureInfo.InvariantCulture, out v)) return v;
            return fallback;
        }
    }
}
