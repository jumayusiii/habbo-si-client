using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
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
        ToggleRow rowGpu, rowThrottle, rowFps, rowDev, rowDiscord;
        FieldBox fieldAppId;
        FlatBtn btnFolder, btnCache, btnPortal, btnCancel, btnSave;
        WinBtn close;

        Rectangle rHead, rAppLabel, rPerfLabel, rTiles, rRows, rNote, rDiscLabel, rStatus;
        bool dragging;
        Point dragAt;
        readonly bool initialGpu, initialThrottle;
        readonly bool initialDiscord;
        readonly string initialAppId;
        readonly ThemeMode originalMode;

        /// <summary>The settings to apply when the sheet reports OK.</summary>
        public ClientSettings Result { get { return working; } }

        /// <summary>True when a renderer switch changed and only a restart can apply it.</summary>
        public bool BrowserArgsChanged { get; private set; }

        /// <summary>True when Discord presence needs to be (re)started after saving.</summary>
        public bool DiscordChanged
        {
            get { return working.Discord != initialDiscord || (working.DiscordAppId ?? "") != initialAppId; }
        }

        public SettingsSheet(ClientSettings current, CoreWebView2 web)
        {
            working = current.Clone();
            core = web;
            initialGpu = current.Gpu;
            initialThrottle = current.NoThrottle;
            initialDiscord = current.Discord;
            initialAppId = current.DiscordAppId ?? "";
            originalMode = current.Mode;

            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            StartPosition = FormStartPosition.CenterParent;
            BackColor = T.Card;
            KeyPreview = true;
            try { Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath); }
            catch { }

            ClientSize = new Size(T.P(560), T.P(676));
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

            rowDiscord = MakeRow(L.T("Discord Rich Presence", "Discord Rich Presence"),
                L.T("Big artwork with the client version, and the room you are in",
                    "Großes Artwork mit Client-Version und deinem Raum"),
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
            close.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(close);
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
            BrowserArgsChanged = rowGpu.On != initialGpu || rowThrottle.On != initialThrottle;
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

        protected override void OnFormClosed(FormClosedEventArgs e)
        {
            DiscordRpc.Changed -= OnDiscordChanged;
            if (DialogResult != DialogResult.OK) T.Apply(originalMode);
            if (Owner != null) Owner.Invalidate(true);
            base.OnFormClosed(e);
        }

        void ApplyPalette()
        {
            BackColor = T.Card;
            foreach (Control c in Controls) c.BackColor = T.Card;
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
            rHead = new Rectangle(0, 0, w, T.P(52));

            close.SetBounds(w - T.P(12) - T.P(40), T.P(10), T.P(40), T.P(30));

            int y = rHead.Bottom + T.P(6);
            rAppLabel = new Rectangle(pad, y, w - pad * 2, T.P(18));
            y += T.P(24);

            int gap = T.P(10);
            int tw = (w - pad * 2 - gap * 2) / 3;
            rTiles = new Rectangle(pad, y, w - pad * 2, T.P(94));
            tDark.SetBounds(rTiles.X, y, tw, rTiles.Height);
            tLight.SetBounds(tDark.Right + gap, y, tw, rTiles.Height);
            tNavy.SetBounds(tLight.Right + gap, y, tw, rTiles.Height);
            y += rTiles.Height + T.P(16);

            rPerfLabel = new Rectangle(pad, y, w - pad * 2, T.P(18));
            y += T.P(24);

            int rowH = T.P(44);
            rRows = new Rectangle(pad, y, w - pad * 2, rowH * 4);
            ToggleRow[] rows = new ToggleRow[] { rowGpu, rowThrottle, rowFps, rowDev };
            for (int i = 0; i < rows.Length; i++)
                rows[i].SetBounds(rRows.X, y + i * rowH, rRows.Width, rowH);
            y += rowH * 4 + T.P(4);

            rNote = new Rectangle(pad, y, w - pad * 2, T.P(16));
            y += T.P(32);

            rDiscLabel = new Rectangle(pad, y, w - pad * 2, T.P(18));
            y += T.P(22);

            rowDiscord.SetBounds(pad, y, w - pad * 2, rowH);
            y += rowH + T.P(6);

            fieldAppId.SetBounds(pad, y, w - pad * 2, T.P(36));
            y += T.P(42);

            rStatus = new Rectangle(pad + T.P(18), y, w - pad * 2 - T.P(18), T.P(16));
            y += T.P(24);

            btnPortal.SetBounds(pad, y, T.P(160), T.P(28));

            int bh = T.P(36);
            int by = ClientSize.Height - pad - bh;
            btnFolder.SetBounds(pad, by, T.P(150), bh);
            btnCache.SetBounds(btnFolder.Right + T.P(8), by, T.P(112), bh);
            btnSave.SetBounds(w - pad - T.P(96), by, T.P(96), bh);
            btnCancel.SetBounds(btnSave.Left - T.P(8) - T.P(96), by, T.P(96), bh);

            ThemeTile[] all = new ThemeTile[] { tDark, tLight, tNavy };
            foreach (ThemeTile t in all) { t.BackColor = T.Card; }
            foreach (ToggleRow r in rows) r.BackColor = T.Card;
            rowDiscord.BackColor = T.Card;
            btnFolder.BackColor = btnCache.BackColor = btnPortal.BackColor = T.Card;
            btnCancel.BackColor = btnSave.BackColor = T.Card;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            g.SmoothingMode = SmoothingMode.AntiAlias;
            using (SolidBrush b = new SolidBrush(T.Card)) g.FillRectangle(b, ClientRectangle);

            T.Str(g, L.T("Settings", "Einstellungen"), T.FPageTitle, T.Text,
                new Rectangle(T.P(22), 0, Width - T.P(90), rHead.Height), T.FL);

            T.Str(g, L.T("APPEARANCE", "ERSCHEINUNGSBILD"), T.FLabel, T.Dim, rAppLabel, T.FL);
            T.Str(g, L.T("PERFORMANCE", "LEISTUNG"), T.FLabel, T.Dim, rPerfLabel, T.FL);
            T.Str(g, L.T("DISCORD", "DISCORD"), T.FLabel, T.Dim, rDiscLabel, T.FL);

            T.Str(g, L.T("Restart the client to apply the renderer switches.",
                         "Für die Renderer-Schalter den Client neu starten."),
                T.FMicro, T.Dim, rNote, T.FL);

            T.Str(g, DiscordStatus(), T.FMicro, T.Dim, rStatus, T.FL);
            using (SolidBrush dot = new SolidBrush(DiscordDot()))
                g.FillEllipse(dot, rStatus.X - T.P(12), rStatus.Y + (rStatus.Height - T.P(6)) / 2, T.P(6), T.P(6));

            T.HLine(g, T.P(22), Width - T.P(22), rHead.Bottom, T.LineSoft);
            T.HLine(g, T.P(22), Width - T.P(22), rRows.Bottom + T.P(2), T.LineSoft);

            using (GraphicsPath rim = T.Round(new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f), T.F(12f)))
            using (Pen pen = new Pen(T.Rim, 1f))
                g.DrawPath(pen, rim);
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
