using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Runtime.InteropServices;
using System.Windows.Forms;
using Microsoft.Web.WebView2.Core;

namespace HabboSiClient
{
    /// <summary>Themed settings sheet: appearance, renderer switches, Discord presence and profile tools.</summary>
    internal class SettingsSheet : Form
    {
        readonly ClientSettings working;
        readonly CoreWebView2 core;

        ThemeTile tDark, tLight, tNavy;
        ToggleRow rowGpu, rowUnlock, rowThrottle, rowFps, rowDev, rowBeta, rowDiscord;
        FieldBox fieldAppId;
        FlatBtn btnFolder, btnCache, btnPortal, btnCancel, btnSave, btnNotes;
        WinBtn close;
        TagPill pill;
        LangSwitch langSwitch;

        Rectangle rHead, rFoot, rAppLabel, rPerfLabel, rTiles, rRows, rNote, rDiscLabel, rStatus, rUpdLabel;
        bool dragging;
        Point dragAt;
        readonly bool initialGpu, initialUnlock, initialThrottle;
        readonly bool initialDiscord;
        readonly string initialAppId;
        readonly ThemeMode originalMode;
        readonly bool originalGerman;

        /// <summary>The settings to apply when the sheet reports OK.</summary>
        public ClientSettings Result { get { return working; } }

        /// <summary>True when a renderer switch changed and only a restart can apply it.</summary>
        public bool BrowserArgsChanged { get; private set; }

        /// <summary>True when Discord presence needs to be (re)started after saving.</summary>
        public bool DiscordChanged
        {
            get { return working.Discord != initialDiscord || (working.DiscordAppId ?? "") != initialAppId; }
        }

        /// <summary>True when the user asked to reopen the current release notes.</summary>
        public bool ReleaseNotesRequested { get; private set; }

        public SettingsSheet(ClientSettings current, CoreWebView2 web)
        {
            working = current.Clone();
            core = web;
            initialGpu = current.Gpu;
            initialUnlock = current.UnlockFps;
            initialThrottle = current.NoThrottle;
            initialDiscord = current.Discord;
            initialAppId = current.DiscordAppId ?? "";
            originalMode = current.Mode;
            originalGerman = current.German;

            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            StartPosition = FormStartPosition.CenterParent;
            BackColor = T.Bg1;
            KeyPreview = true;
            try { Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath); }
            catch { }

            ClientSize = new Size(T.P(900), T.P(604));
            BuildUi();
            LayoutAll();
            SheetFade.Attach(this);
            DiscordRpc.Changed += OnDiscordChanged;
        }

        void BuildUi()
        {
            tDark = new ThemeTile(); tDark.Mode = ThemeMode.Dark;
            tLight = new ThemeTile(); tLight.Mode = ThemeMode.Light;
            tNavy = new ThemeTile(); tNavy.Mode = ThemeMode.Navy;
            ThemeTile[] tiles = new ThemeTile[] { tDark, tLight, tNavy };
            foreach (ThemeTile t in tiles)
            {
                t.Selected = t.Mode == working.Mode;
                t.Click += delegate (object sender, EventArgs e) { PickTheme(((ThemeTile)sender).Mode); };
                Controls.Add(t);
            }

            rowGpu = MakeRow(L.T("Hardware acceleration", "Hardware-Beschleunigung"),
                L.T("GPU rasterisation for the hotel renderer", "GPU-Rasterisierung für den Hotel-Renderer"),
                working.Gpu);
            rowGpu.Restart = true;
            rowGpu.Changed += delegate { working.Gpu = rowGpu.On; MarkArgs(); };

            rowUnlock = MakeRow(L.T("Unlock frame rate", "Bildrate freigeben"),
                L.T("Uncapped renderer for 60–120 FPS", "Renderer ohne 60-FPS-Grenze"),
                working.UnlockFps);
            rowUnlock.Restart = true;
            rowUnlock.Changed += delegate { working.UnlockFps = rowUnlock.On; MarkArgs(); };

            rowThrottle = MakeRow(L.T("Keep running in the background", "Im Hintergrund weiterlaufen"),
                L.T("No timer throttling while the window is covered", "Kein Timer-Drosseln bei verdecktem Fenster"),
                working.NoThrottle);
            rowThrottle.Restart = true;
            rowThrottle.Changed += delegate { working.NoThrottle = rowThrottle.On; MarkArgs(); };

            rowFps = MakeRow(L.T("Show FPS", "FPS anzeigen"),
                L.T("Frame rate readout in the footer", "Bildrate in der Fußzeile"),
                working.ShowFps);
            rowFps.Changed += delegate { working.ShowFps = rowFps.On; };

            rowDev = MakeRow(L.T("Developer tools", "Entwicklerwerkzeuge"),
                L.T("Open the inspector with F12", "Den Inspector mit F12 öffnen"),
                working.DevTools);
            rowDev.Changed += delegate { working.DevTools = rowDev.On; };

            rowBeta = MakeRow(L.T("Beta channel", "Beta-Kanal"),
                L.T("Follow GitHub pre-releases", "GitHub-Vorabversionen folgen"),
                working.UpdateChannel == "beta");
            rowBeta.Changed += delegate { working.UpdateChannel = rowBeta.On ? "beta" : "stable"; };

            btnNotes = new FlatBtn();
            btnNotes.Label = L.T("Release notes", "Release-Notizen");
            btnNotes.Icon = "info";
            btnNotes.Click += delegate
            {
                ReleaseNotesRequested = true;
                DialogResult = DialogResult.Cancel;
                Close();
            };
            Controls.Add(btnNotes);

            rowDiscord = MakeRow(L.T("Discord Rich Presence", "Discord Rich Presence"),
                L.T("Big artwork, client version and your room",
                    "Großes Artwork, Version und dein Raum"),
                working.Discord);
            rowDiscord.Changed += delegate { working.Discord = rowDiscord.On; Invalidate(); };

            fieldAppId = new FieldBox();
            fieldAppId.SetPlaceholder(L.T("Discord application ID (17–20 digits)",
                                          "Discord-Application-ID (17–20 Ziffern)"));
            fieldAppId.Value = working.DiscordAppId;
            fieldAppId.ValueChanged += delegate { Invalidate(); };
            Controls.Add(fieldAppId);

            btnPortal = new FlatBtn();
            btnPortal.Label = L.T("Developer Portal", "Entwickler-Portal");
            btnPortal.Click += delegate { OpenPortal(); };
            Controls.Add(btnPortal);

            btnFolder = new FlatBtn();
            btnFolder.Label = L.T("Open profile folder", "Profilordner");
            btnFolder.Icon = "folder";
            btnFolder.Click += delegate
            {
                try { Process.Start("explorer.exe", Store.Dir); } catch { }
            };
            Controls.Add(btnFolder);

            btnCache = new FlatBtn();
            btnCache.Label = L.T("Clear cache", "Cache leeren");
            btnCache.Click += delegate { ClearCache(); };
            Controls.Add(btnCache);

            btnCancel = new FlatBtn();
            btnCancel.Label = L.T("Cancel", "Abbrechen");
            btnCancel.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(btnCancel);

            btnSave = new FlatBtn();
            btnSave.Label = L.T("Save", "Speichern");
            btnSave.Primary = true;
            btnSave.Click += delegate { ConfirmSave(); };
            Controls.Add(btnSave);

            close = new WinBtn();
            close.Icon = "close";
            close.BackColor = T.Bg1;
            close.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(close);

            langSwitch = new LangSwitch();
            langSwitch.German = working.German;
            langSwitch.Changed += delegate { SetLanguage(langSwitch.German); };
            Controls.Add(langSwitch);

            pill = new TagPill();
            pill.Caption = "SETTINGS";
            pill.BackColor = T.Bg1;
            Controls.Add(pill);
        }

        ToggleRow MakeRow(string label, string note, bool on)
        {
            ToggleRow r = new ToggleRow();
            r.Label = label;
            r.Note = note;
            r.On = on;
            Controls.Add(r);
            return r;
        }

        void MarkArgs()
        {
            // only a renderer switch needs a restart; the rest apply on the spot
            BrowserArgsChanged = rowGpu.On != initialGpu || rowUnlock.On != initialUnlock ||
                                 rowThrottle.On != initialThrottle;
        }

        void PickTheme(ThemeMode mode)
        {
            working.Mode = mode;
            tDark.Selected = mode == ThemeMode.Dark;
            tLight.Selected = mode == ThemeMode.Light;
            tNavy.Selected = mode == ThemeMode.Navy;
            foreach (ThemeTile t in new ThemeTile[] { tDark, tLight, tNavy }) t.Invalidate();

            // preview the palette on the sheet and on the window behind it; Cancel puts it back
            T.Apply(mode);
            ApplyPalette();
            Invalidate(true);
            if (Owner != null) Owner.Invalidate(true);
        }

        void SetLanguage(bool de)
        {
            working.German = de;
            L.De = de;

            rowGpu.Label = L.T("Hardware acceleration", "Hardware-Beschleunigung");
            rowGpu.Note = L.T("GPU rasterisation for the hotel renderer", "GPU-Rasterisierung für den Hotel-Renderer");
            rowUnlock.Label = L.T("Unlock frame rate", "Bildrate freigeben");
            rowUnlock.Note = L.T("Uncapped renderer for 60–120 FPS", "Renderer ohne 60-FPS-Grenze");
            rowThrottle.Label = L.T("Keep running in the background", "Im Hintergrund weiterlaufen");
            rowThrottle.Note = L.T("No timer throttling while the window is covered", "Kein Timer-Drosseln bei verdecktem Fenster");
            rowFps.Label = L.T("Show FPS", "FPS anzeigen");
            rowFps.Note = L.T("Frame rate readout in the footer", "Bildrate in der Fußzeile");
            rowDev.Label = L.T("Developer tools", "Entwicklerwerkzeuge");
            rowDev.Note = L.T("Open the inspector with F12", "Den Inspector mit F12 öffnen");
            rowBeta.Label = L.T("Beta channel", "Beta-Kanal");
            rowBeta.Note = L.T("Follow GitHub pre-releases", "GitHub-Vorabversionen folgen");
            rowDiscord.Label = L.T("Discord Rich Presence", "Discord Rich Presence");
            rowDiscord.Note = L.T("Big artwork, client version and your room",
                "Großes Artwork, Version und dein Raum");
            fieldAppId.SetPlaceholder(L.T("Discord application ID (17–20 digits)",
                                          "Discord-Application-ID (17–20 Ziffern)"));
            btnPortal.Label = L.T("Developer Portal", "Entwickler-Portal");
            btnNotes.Label = L.T("Release notes", "Release-Notizen");
            btnFolder.Label = L.T("Open profile folder", "Profilordner");
            btnCache.Label = L.T("Clear cache", "Cache leeren");
            btnCancel.Label = L.T("Cancel", "Abbrechen");
            btnSave.Label = L.T("Save", "Speichern");

            foreach (ToggleRow r in new ToggleRow[] { rowGpu, rowUnlock, rowThrottle, rowFps, rowDev, rowBeta, rowDiscord })
                r.Invalidate();
            foreach (ThemeTile t in new ThemeTile[] { tDark, tLight, tNavy }) t.Invalidate();
            foreach (FlatBtn b in new FlatBtn[] { btnPortal, btnFolder, btnCache, btnCancel, btnSave, btnNotes })
                b.Invalidate();
            fieldAppId.Invalidate();
            Invalidate(true);
        }

        void ConfirmSave()
        {
            string id = fieldAppId.Value.Trim();
            if (id.Length > 0 && !DiscordRpc.IsValidAppId(id))
            {
                MessageSheetForm.Error(this,
                    L.T("That is not an application ID", "Das ist keine Application-ID"),
                    L.T("A Discord application ID is 17 to 20 digits. Copy it from the Developer Portal.",
                       "Eine Discord-Application-ID besteht aus 17 bis 20 Ziffern. Kopiere sie aus dem Entwickler-Portal."),
                    id);
                return;
            }
            working.DiscordAppId = id;
            working.Discord = rowDiscord.On;
            DialogResult = DialogResult.OK;
            Close();
        }

        void OpenPortal()
        {
            string id = fieldAppId.Value.Trim();
            string url = DiscordRpc.IsValidAppId(id)
                ? "https://discord.com/developers/applications/" + id + "/rich-presence/assets"
                : "https://discord.com/developers/applications";
            try { Process.Start(new ProcessStartInfo(url) { UseShellExecute = true }); }
            catch { }
        }

        void OnDiscordChanged()
        {
            if (IsDisposed || !IsHandleCreated) return;
            try { BeginInvoke(new Action(delegate { Invalidate(); })); }
            catch { }
        }

        [DllImport("dwmapi.dll")]
        static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);

        protected override void OnShown(EventArgs e)
        {
            base.OnShown(e);
            try
            {
                // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_DONOTROUND = 1.
                // The sheet is drawn square, so DWM must not clip rounded bites out of it.
                int square = 1;
                DwmSetWindowAttribute(Handle, 33, ref square, sizeof(int));
            }
            catch { }
        }

        protected override void OnFormClosed(FormClosedEventArgs e)
        {
            DiscordRpc.Changed -= OnDiscordChanged;
            if (DialogResult != DialogResult.OK)
            {
                L.De = originalGerman;
                T.Apply(originalMode);
            }
            if (Owner != null) Owner.Invalidate(true);
            base.OnFormClosed(e);
        }

        void ApplyPalette()
        {
            BackColor = T.Bg1;
            foreach (Control c in Controls) c.BackColor = T.Bg1;
            fieldAppId.ApplyTheme();
            Invalidate(true);
        }

        string DiscordStatus()
        {
            switch (DiscordRpc.Status)
            {
                case "connected": return L.T("Connected", "Verbunden");
                case "connecting": return L.T("Connecting…", "Verbinde…");
                case "no-discord": return L.T("Discord is not running", "Discord läuft nicht");
                case "error": return L.T("Discord refused the connection", "Discord hat die Verbindung abgelehnt");
            }
            if (working.Discord && !DiscordRpc.IsValidAppId(fieldAppId.Value))
                return L.T("Paste your Discord application ID", "Discord-Application-ID einfügen");
            return L.T("Off", "Aus");
        }

        Color DiscordDot()
        {
            if (DiscordRpc.Status == "connected") return T.Gold;
            if (DiscordRpc.Status == "connecting") return T.Muted;
            return T.Dim;
        }

        async void ClearCache()
        {
            if (core == null) return;
            bool ok = MessageSheetForm.Confirm(this,
                L.T("Clear the client cache?", "Client-Cache leeren?"),
                L.T("Cached assets and the saved hotel session are removed. You will have to log in again.",
                   "Zwischengespeicherte Assets und die gespeicherte Hotel-Sitzung werden entfernt. Du musst dich neu anmelden."),
                Store.ProfileDir,
                L.T("Clear", "Leeren"), L.T("Cancel", "Abbrechen"), null);
            if (!ok) return;

            btnCache.Label = L.T("Clearing…", "Leeren…");
            btnCache.Invalidate();
            try
            {
                await core.Profile.ClearBrowsingDataAsync();
                MessageSheetForm.Error(this, L.T("Cache cleared", "Cache geleert"),
                    L.T("The next reload downloads the hotel's assets again.",
                       "Beim nächsten Neuladen werden die Assets des Hotels erneut geladen."), "");
            }
            catch (Exception ex)
            {
                MessageSheetForm.Error(this, L.T("Could not clear the cache", "Cache konnte nicht geleert werden"), ex.Message, "");
            }
            btnCache.Label = L.T("Clear cache", "Cache leeren");
            btnCache.Invalidate();
        }

        void LayoutAll()
        {
            int pad = T.P(22);
            int w = ClientSize.Width;
            int h = ClientSize.Height;
            rHead = new Rectangle(0, 0, w, T.P(64));
            rFoot = new Rectangle(0, h - T.P(30), w, T.P(30));

            int wmW = T.Wordmark != null ? T.P(T.Wordmark.Width) : 0;
            int pillW = T.Meas("SETTINGS", T.FLabel).Width + T.P(18);
            int pillH = T.P(20);
            pill.SetBounds(T.P(22) + wmW + T.P(14), (rHead.Height - pillH) / 2, pillW, pillH);
            close.SetBounds(w - T.P(10) - T.P(42), (rHead.Height - T.P(30)) / 2, T.P(42), T.P(30));
            int langW = T.P(74), langH = T.P(28);
            langSwitch.SetBounds(close.Left - T.P(12) - langW, (rHead.Height - langH) / 2, langW, langH);

            int gap = T.P(26);
            int leftW = T.P(470);
            int rightX = pad + leftW + gap;
            int rightW = w - pad - rightX;
            int rowH = T.P(44);

            int y = rHead.Bottom + T.P(18);
            rAppLabel = new Rectangle(pad, y, leftW, T.P(18));
            y += T.P(24);

            rTiles = new Rectangle(pad, y, leftW, T.P(96));
            int tileGap = T.P(10);
            int tw = (leftW - tileGap * 2) / 3;
            tDark.SetBounds(rTiles.X, y, tw, rTiles.Height);
            tLight.SetBounds(tDark.Right + tileGap, y, tw, rTiles.Height);
            tNavy.SetBounds(tLight.Right + tileGap, y, tw, rTiles.Height);
            y += rTiles.Height + T.P(18);

            rPerfLabel = new Rectangle(pad, y, leftW, T.P(18));
            y += T.P(24);

            ToggleRow[] rows = new ToggleRow[] { rowGpu, rowUnlock, rowThrottle, rowFps, rowDev };
            rRows = new Rectangle(pad, y, leftW, rowH * rows.Length);
            for (int i = 0; i < rows.Length; i++)
                rows[i].SetBounds(rRows.X, y + i * rowH, rRows.Width, rowH);
            y += rRows.Height + T.P(6);
            rNote = new Rectangle(pad, y, leftW, T.P(16));

            int ry = rHead.Bottom + T.P(18);
            rDiscLabel = new Rectangle(rightX, ry, rightW, T.P(18));
            ry += T.P(24);

            rowDiscord.SetBounds(rightX, ry, rightW, rowH);
            ry += rowH + T.P(8);

            fieldAppId.SetBounds(rightX, ry, rightW, T.P(36));
            ry += T.P(42);

            rStatus = new Rectangle(rightX + T.P(18), ry, rightW - T.P(18), T.P(16));
            ry += T.P(24);

            btnPortal.SetBounds(rightX, ry, T.P(160), T.P(28));

            ry += T.P(40);
            rUpdLabel = new Rectangle(rightX, ry, rightW, T.P(18));
            ry += T.P(24);
            rowBeta.SetBounds(rightX, ry, rightW, rowH);
            btnNotes.SetBounds(rightX, rowBeta.Bottom + T.P(8), T.P(152), T.P(28));

            int bh = T.P(36);
            int by = rFoot.Y - T.P(16) - bh;
            btnFolder.SetBounds(pad, by, T.P(150), bh);
            btnCache.SetBounds(btnFolder.Right + T.P(8), by, T.P(112), bh);
            btnSave.SetBounds(w - pad - T.P(96), by, T.P(96), bh);
            btnCancel.SetBounds(btnSave.Left - T.P(8) - T.P(96), by, T.P(96), bh);

            foreach (Control c in Controls) if (c != fieldAppId) c.BackColor = T.Bg1;
            fieldAppId.ApplyTheme();
            Invalidate(true);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            T.PaintBackdrop(g, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            if (T.Wordmark != null)
            {
                int wmH = T.P(T.Wordmark.Height);
                int wmW = T.P(T.Wordmark.Width);
                int wmY = rHead.Y + (rHead.Height - wmH) / 2;
                float scale = T.P(1f);
                bool whole = Math.Abs(scale - (float)Math.Round(scale)) < 0.01f;
                g.InterpolationMode = whole ? InterpolationMode.NearestNeighbor : InterpolationMode.HighQualityBicubic;
                g.PixelOffsetMode = whole ? PixelOffsetMode.Half : PixelOffsetMode.HighQuality;
                g.DrawImage(T.Wordmark, new Rectangle(T.P(22), wmY, wmW, wmH));
            }

            T.HLine(g, 0, Width, rHead.Bottom - 1, T.LineSoft);
            T.HLine(g, 0, Width, rFoot.Y, T.LineSoft);

            T.Str(g, L.T("APPEARANCE", "ERSCHEINUNGSBILD"), T.FLabel, T.Dim, rAppLabel, T.FL);
            T.Str(g, L.T("PERFORMANCE", "LEISTUNG"), T.FLabel, T.Dim, rPerfLabel, T.FL);
            T.Str(g, L.T("DISCORD", "DISCORD"), T.FLabel, T.Dim, rDiscLabel, T.FL);
            T.Str(g, L.T("UPDATES", "UPDATES"), T.FLabel, T.Dim, rUpdLabel, T.FL);

            T.Str(g, L.T("Restart the client to apply the renderer switches.",
                         "Für die Renderer-Schalter den Client neu starten."),
                T.FMicro, T.Dim, rNote, T.FL);

            T.Str(g, DiscordStatus(), T.FMicro, T.Dim, rStatus, T.FL);
            using (SolidBrush dot = new SolidBrush(DiscordDot()))
                g.FillEllipse(dot, rStatus.X - T.P(12), rStatus.Y + (rStatus.Height - T.P(6)) / 2, T.P(6), T.P(6));

            T.VLine(g, rDiscLabel.X - T.P(13), rHead.Bottom + T.P(18), rFoot.Y - T.P(16), T.LineSoft);

            using (Pen pen = new Pen(T.Rim, 1f))
                g.DrawRectangle(pen, 0.5f, 0.5f, Width - 1f, Height - 1f);
        }

        protected override void OnMouseDown(MouseEventArgs e)
        {
            base.OnMouseDown(e);
            if (e.Button == MouseButtons.Left && e.Y < rHead.Bottom)
            {
                dragging = true;
                dragAt = e.Location;
            }
        }

        protected override void OnMouseMove(MouseEventArgs e)
        {
            base.OnMouseMove(e);
            if (!dragging) return;
            Point p = PointToScreen(e.Location);
            Location = new Point(p.X - dragAt.X, p.Y - dragAt.Y);
        }

        protected override void OnMouseUp(MouseEventArgs e)
        {
            dragging = false;
            base.OnMouseUp(e);
        }

        protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
        {
            if (keyData == Keys.Escape) { DialogResult = DialogResult.Cancel; Close(); return true; }
            if (keyData == Keys.Enter && !fieldAppId.Box.Focused) { ConfirmSave(); return true; }
            return base.ProcessCmdKey(ref msg, keyData);
        }
    }
}
