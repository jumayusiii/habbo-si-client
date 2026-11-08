using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Windows.Forms;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace HabboSiClient
{
    /// <summary>
    /// The client window: the original borderless chrome and palette, with the hotel's own
    /// web client hosted in Edge WebView2 underneath it.
    /// </summary>
    internal class ClientForm : Form, IMessageFilter
    {
        ClientSettings settings;

        WebView2 web;
        CoreWebView2 core;

        WinBtn wbMin, wbMax, wbClose;
        LinkChip chipReload, chipSettings;
        IconBtn zoomOut, zoomIn;
        TagPill pill;
#if BETA
        TagPill betaPill;
#endif
        readonly ToolTip tips = new ToolTip();

        Rectangle rBar, rFooter, rContent, rZoom;
        bool fullscreen;
        FormWindowState prevState = FormWindowState.Normal;
        Rectangle prevBounds;

        string status = "";
        string toast = "";
        bool loading;
        float phase;
        int fps = -1;
        bool smokeStarted;

        readonly Timer anim = new Timer();
        readonly Timer fpsTimer = new Timer();
        readonly Timer toastTimer = new Timer();
        readonly Timer smokeTimer = new Timer();
        readonly Timer liveTimer = new Timer();
        readonly Timer roomTimer = new Timer();
        readonly Timer probeTimer = new Timer();
        bool liveStarted;
        readonly List<CoreWebView2Frame> frames = new List<CoreWebView2Frame>();
        string currentRoom = "";
        bool presenceAllowed;
        bool noticeOpen;
        readonly Stopwatch clock = Stopwatch.StartNew();

        bool lastNavOk;
        CoreWebView2WebErrorStatus lastNavStatus;
        int lastNavHttp;
        string lastNavUrl = "";
        long lastNavMs;
        DateTime lastErrorShown = DateTime.MinValue;

        // One rAF counter, installed before the hotel's own scripts run. It only counts frames;
        // the client polls the number once a second, so the page pays almost nothing for it.
        const string FpsScript =
            "(function(){if(window.__hsc)return;window.__hsc={fps:-1,n:0,t:0};var started=0;" +
            "requestAnimationFrame(function loop(ts){if(!started){started=ts;window.__hsc.t=ts;}" +
            "window.__hsc.n++;var d=ts-window.__hsc.t;" +
            "if(d>=1000){window.__hsc.fps=Math.round(window.__hsc.n*1000/d);window.__hsc.n=0;window.__hsc.t=ts;}" +
            "requestAnimationFrame(loop);});})();";

        public ClientForm(ClientSettings s)
        {
            settings = s;

            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FormBorderStyle = FormBorderStyle.None;
            Text = AppInfo.Name + " " + AppInfo.Version;
            BackColor = T.Bg1;
            DoubleBuffered = true;
            KeyPreview = true;
            try { Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath); }
            catch { }

            Rectangle wa = Screen.PrimaryScreen.WorkingArea;
            MinimumSize = new Size(
                Math.Min(T.P(980), (int)(wa.Width * 0.9f)),
                Math.Min(T.P(620), (int)(wa.Height * 0.9f)));
            int w = Math.Min(settings.Width, (int)(wa.Width * 0.94f));
            int h = Math.Min(settings.Height, (int)(wa.Height * 0.92f));
            ClientSize = new Size(Math.Max(T.P(980), w), Math.Max(T.P(620), h));
            MaximizedBounds = wa;
            StartPosition = FormStartPosition.CenterScreen;

            BuildUi();
            LayoutAll();
            status = L.T("Starting…", "Startet…");
            if (Program.ShotPath != null) status = L.T("Preview render", "Vorschau-Render");
            Application.AddMessageFilter(this);

            anim.Interval = 16;
            anim.Tick += delegate
            {
                phase += 0.045f;
                if (phase > 1.6f) phase = -0.6f;
                Invalidate(new Rectangle(0, rBar.Bottom - 2, Width, T.P(4)));
            };

            fpsTimer.Interval = 1000;
            fpsTimer.Tick += delegate { PollFps(); };

            toastTimer.Interval = 3800;
            toastTimer.Tick += delegate { toastTimer.Stop(); toast = ""; Invalidate(rFooter); };

            smokeTimer.Interval = 1600;
            smokeTimer.Tick += delegate { smokeTimer.Stop(); FinishSmoke(); };

            liveTimer.Interval = 10000;
            liveTimer.Tick += delegate
            {
                liveTimer.Stop();
                CaptureWindow(Program.ShotLive);
                Close();
            };

            // Render runs must not touch the user's Discord session.
            presenceAllowed = Program.ShotPath == null && Program.SmokeLog == null &&
                               Program.ShotLive == null && Program.ProbePath == null;
            roomTimer.Interval = 4000;
            roomTimer.Tick += delegate { PollRoom(); };
            probeTimer.Interval = 12000;
            probeTimer.Tick += delegate { RunProbe(); };
        }

        void BuildUi()
        {
            wbMin = new WinBtn();
            wbMin.Icon = "min";
            wbMin.Click += delegate { WindowState = FormWindowState.Minimized; };
            Controls.Add(wbMin);

            wbMax = new WinBtn();
            wbMax.Icon = "max";
            wbMax.Click += delegate { ToggleMaximize(); };
            Controls.Add(wbMax);

            wbClose = new WinBtn();
            wbClose.Icon = "close";
            wbClose.Click += delegate { Close(); };
            Controls.Add(wbClose);

            chipReload = new LinkChip();
            chipReload.Icon = "rotate";
            chipReload.Click += delegate { Reload(); };
            Controls.Add(chipReload);

            chipSettings = new LinkChip();
            chipSettings.Icon = "gear";
            chipSettings.Click += delegate { OpenSettings(); };
            Controls.Add(chipSettings);

            zoomOut = new IconBtn();
            zoomOut.Icon = "minus";
            zoomOut.Segmented = true;
            zoomOut.Click += delegate { ZoomBy(-0.1f); };
            Controls.Add(zoomOut);

            zoomIn = new IconBtn();
            zoomIn.Icon = "plus";
            zoomIn.Segmented = true;
            zoomIn.Click += delegate { ZoomBy(0.1f); };
            Controls.Add(zoomIn);

            pill = new TagPill();
            pill.Caption = "CLIENT";
            pill.Interactive = true;
            Controls.Add(pill);

#if BETA
            betaPill = new TagPill();
            betaPill.Caption = "BETA";
            betaPill.Filled = true;
            Controls.Add(betaPill);
#endif

            tips.InitialDelay = 420;
            tips.ReshowDelay = 120;
            tips.AutoPopDelay = 8000;
            tips.SetToolTip(chipReload, ReloadTip());
            tips.SetToolTip(chipSettings, SettingsTip());
            tips.SetToolTip(pill, ClientTip());
#if BETA
            tips.SetToolTip(betaPill, L.T("Beta channel build", "Beta-Kanal-Build"));
#endif

            if (Program.ShotPath == null)
            {
                web = new WebView2();
                web.DefaultBackgroundColor = T.Bg1;
                Controls.Add(web);
                web.SendToBack();
            }
        }

        // ------------------------------------------------------------------ layout
        void LayoutAll()
        {
            if (wbClose == null) return;   // a resize can arrive before the chrome exists
            int w = ClientSize.Width, h = ClientSize.Height;
            int barH = fullscreen ? 0 : T.P(64);
            int footH = fullscreen ? 0 : T.P(30);
            rBar = new Rectangle(0, 0, w, barH);
            rFooter = new Rectangle(0, h - footH, w, footH);
            rContent = new Rectangle(0, barH, w, Math.Max(0, h - barH - footH));

            if (web != null) web.SetBounds(rContent.X, rContent.Y, rContent.Width, rContent.Height);

            bool show = !fullscreen;
            wbMin.Visible = wbMax.Visible = wbClose.Visible = show;
            chipReload.Visible = chipSettings.Visible = show;
            zoomIn.Visible = zoomOut.Visible = show;
            pill.Visible = show;
#if BETA
            betaPill.Visible = show;
#endif
            if (!show) return;

            int padR = T.P(10), bw = T.P(42), bh = T.P(30), by = (barH - bh) / 2;
            wbClose.SetBounds(w - padR - bw, by, bw, bh);
            wbMax.SetBounds(wbClose.Left - bw, by, bw, bh);
            wbMin.SetBounds(wbMax.Left - bw, by, bw, bh);

            int chipH = T.P(28), cy = (barH - chipH) / 2;

            // Both app-bar chips are icon-only and share the same width.
            int setW = T.P(32);
            chipSettings.SetBounds(wbMin.Left - T.P(12) - setW, cy, setW, chipH);

            // Icon-only: a square target that only lights up under the pointer.
            int relW = T.P(32);
            chipReload.SetBounds(chipSettings.Left - T.P(8) - relW, cy, relW, chipH);

            int b = T.P(26);
            int zw = T.P(42);
            int groupW = b * 2 + zw + T.P(8);
            int gx = chipReload.Left - T.P(12) - groupW;
            zoomOut.SetBounds(gx, cy + (chipH - b) / 2, b, b);
            rZoom = new Rectangle(zoomOut.Right, cy, zw, chipH);
            zoomIn.SetBounds(rZoom.Right, cy + (chipH - b) / 2, b, b);

            int wmH = T.Wordmark != null ? T.P(T.Wordmark.Height) : 0;
            int wmW = T.Wordmark != null ? T.P(T.Wordmark.Width) : 0;
            int pillW = T.Meas("CLIENT", T.FLabel).Width + T.P(18);
            int pillH = T.P(20);
            int pillX = T.P(22) + wmW + T.P(14);
            pill.SetBounds(pillX, (barH - pillH) / 2, pillW, pillH);
#if BETA
            int betaW = T.Meas("BETA", T.FLabel).Width + T.P(14);
            betaPill.SetBounds(pill.Right + T.P(7), (barH - pillH) / 2, betaW, pillH);
#endif
        }

        // ------------------------------------------------------------------ painting
        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            T.PaintBackdrop(g, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            if (fullscreen)
            {
                DrawRim(g);
                return;
            }

            T.HLine(g, 0, Width, rBar.Bottom - 1, T.LineSoft);

            if (T.Wordmark != null)
            {
                int wmH = T.P(T.Wordmark.Height);
                int wmW = T.P(T.Wordmark.Width);
                int wmY = rBar.Y + (rBar.Height - wmH) / 2;
                float scale = T.P(1f);
                bool whole = Math.Abs(scale - (float)Math.Round(scale)) < 0.01f;
                g.InterpolationMode = whole ? InterpolationMode.NearestNeighbor : InterpolationMode.HighQualityBicubic;
                g.PixelOffsetMode = whole ? PixelOffsetMode.Half : PixelOffsetMode.HighQuality;
                g.DrawImage(T.Wordmark, new Rectangle(T.P(22), wmY, wmW, wmH));
            }

            T.Str(g, Math.Round(settings.Zoom * 100) + "%", T.FMicro, T.Muted, rZoom, T.FC);

            if (loading)
            {
                int seg = T.P(190);
                int x = (int)(phase * (Width + seg)) - seg;
                Rectangle clip = new Rectangle(0, rBar.Bottom - 2, Width, T.P(2));
                GraphicsState st = g.Save();
                g.SetClip(clip);
                using (SolidBrush b = new SolidBrush(T.Gold))
                    g.FillRectangle(b, x, clip.Y, seg, clip.Height);
                g.Restore(st);
            }

            DrawFooter(g);
            if (Program.ShotPath != null) DrawViewportPlaceholder(g);
            DrawRim(g);
        }

        void DrawRim(Graphics g)
        {
            using (GraphicsPath rim = T.Round(new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f), T.F(12f)))
            using (Pen pen = new Pen(T.Rim, 1f))
                g.DrawPath(pen, rim);
        }

        void DrawFooter(Graphics g)
        {
            if (fullscreen) return;
            T.HLine(g, 0, Width, rFooter.Y, T.LineSoft);

            int pad = T.P(18);
            int y = rFooter.Y;
            int right = Width - pad;

            if (toast.Length > 0)
            {
                int tw = T.Meas(toast, T.FMicro).Width + T.P(18);
                Rectangle tr = new Rectangle(right - tw, y + T.P(6), tw, rFooter.Height - T.P(12));
                T.FillRound(g, tr, T.F(7f), T.ToastBg);
                T.Str(g, toast, T.FMicro, T.ToastText, tr, T.FC);
                right -= tw + T.P(12);
            }

            string version = "v" + AppInfo.Version;
            int vw = T.Meas(version, T.FMicro).Width;
            int gap = T.P(8);
            if (settings.ShowFps)
            {
                string f = fps >= 0 ? fps + " FPS" : "— FPS";
                int fw = T.Meas(f, T.FMicro).Width;
                T.Str(g, f, T.FMicro, fps >= 60 ? T.Green : (fps >= 0 ? T.Muted : T.Dim),
                    new Rectangle(right - fw, y, fw, rFooter.Height), T.FR);
                right -= fw + gap;
            }

            T.Str(g, version, T.FMicro, T.Dim,
                new Rectangle(right - vw, y, vw, rFooter.Height), T.FR);
        }

        void DrawViewportPlaceholder(Graphics g)
        {
            T.FillRound(g, rContent, 0f, T.Inset);
            int cx = rContent.X + rContent.Width / 2;
            int cy = rContent.Y + rContent.Height / 2;

            if (T.Logo != null)
            {
                int s = T.P(56);
                g.InterpolationMode = InterpolationMode.NearestNeighbor;
                g.PixelOffsetMode = PixelOffsetMode.Half;
                g.DrawImage(T.Logo, new Rectangle(cx - s / 2, cy - T.P(74), s, s));
            }

            T.Str(g, "habbo.si/client", T.FPageTitle, T.Text,
                new Rectangle(rContent.X, cy - T.P(8), rContent.Width, T.P(28)), T.FC);
            T.Str(g, L.T("WebView2 viewport is not drawn in this render",
                         "WebView2-Fläche wird in diesem Render nicht gezeichnet"),
                T.FSmall, T.Dim, new Rectangle(rContent.X, cy + T.P(20), rContent.Width, T.P(20)), T.FC);
        }


        // ------------------------------------------------------------------ webview
        protected override async void OnShown(EventArgs e)
        {
            base.OnShown(e);
            if (settings.Maximized) WindowState = FormWindowState.Maximized;
            if (Program.ShotPath != null) return;
            // captures and diagnostics must stay deterministic, so no network checks there
            bool deterministic = Program.SmokeLog != null || Program.ShotLive != null || Program.ProbePath != null;
            if (!deterministic) CheckForUpdates();
            await InitWeb();
            if (!deterministic) MaybeShowWhatsNew();
        }

        /// <summary>
        /// One quiet GitHub lookup per launch. Nothing is downloaded here: a newer tag opens a
        /// themed sheet whose action hands the release page to the default browser.
        /// </summary>
        async void CheckForUpdates()
        {
            bool beta = settings.UpdateChannel == "beta";
            UpdateInfo info = await Task.Run(() => UpdateCheck.Latest(beta));
            if (!UpdateCheck.HasUpdate(info, beta)) return;
            if (IsDisposed || !IsHandleCreated) return;
            try
            {
                BeginInvoke(new Action(delegate
                {
                    if (IsDisposed || !BeginNotice()) return;
                    try
                    {
                        string body = L.T(
                            AppInfo.Name + " " + info.Display + " is available. You are running " + AppInfo.Version + ".",
                            AppInfo.Name + " " + info.Display + " ist verfügbar. Du verwendest " + AppInfo.Version + ".");
                        bool open = MessageSheetForm.Notice(this,
                            L.T("Update available", "Update verfügbar"),
                            body, info.PageUrl,
                            L.T("Open release", "Release öffnen"),
                            L.T("Later", "Später"));
                        if (open) OpenExternal(info.PageUrl);
                    }
                    finally { EndNotice(); }
                }));
            }
            catch { }
        }

        /// <summary>Only one themed notice at a time; a skipped note retries on the next launch.</summary>
        bool BeginNotice()
        {
            if (noticeOpen) return false;
            noticeOpen = true;
            return true;
        }

        void EndNotice() { noticeOpen = false; }

        /// <summary>Shows the current build's release notes once per profile.</summary>
        async void MaybeShowWhatsNew()
        {
            if (settings.SeenRelease == AppInfo.ReleaseTag) return;
            ReleaseNotes notes = await Task.Run(() => UpdateCheck.Notes(AppInfo.ReleaseTag));
            if (notes == null || string.IsNullOrEmpty(notes.Body)) return;
            if (IsDisposed || !IsHandleCreated) return;
            try
            {
                BeginInvoke(new Action(delegate
                {
                    if (IsDisposed || !BeginNotice()) return;
                    try
                    {
                        bool open = WhatsNewForm.ShowSheet(this, notes);
                        settings.SeenRelease = AppInfo.ReleaseTag;
                        Store.Save(settings);
                        if (open && !string.IsNullOrEmpty(notes.PageUrl)) OpenExternal(notes.PageUrl);
                    }
                    finally { EndNotice(); }
                }));
            }
            catch { }
        }

        /// <summary>Loads and shows the running build's notes even after the startup sheet was dismissed.</summary>
        async void ShowReleaseNotes()
        {
            ReleaseNotes notes;
            try { notes = await Task.Run(() => UpdateCheck.Notes(AppInfo.ReleaseTag)); }
            catch { notes = null; }

            if (IsDisposed || !IsHandleCreated) return;
            try
            {
                BeginInvoke(new Action(delegate
                {
                    if (IsDisposed || !BeginNotice()) return;
                    try
                    {
                        if (notes == null || string.IsNullOrEmpty(notes.Body))
                        {
                            MessageSheetForm.Error(this,
                                L.T("Release notes unavailable", "Release-Notizen nicht verfügbar"),
                                L.T("The release notes could not be loaded. Check your connection and try again.",
                                   "Die Release-Notizen konnten nicht geladen werden. Verbindung prüfen und erneut versuchen."),
                                AppInfo.ReleaseTag);
                            return;
                        }
                        bool open = WhatsNewForm.ShowSheet(this, notes);
                        settings.SeenRelease = AppInfo.ReleaseTag;
                        Store.Save(settings);
                        if (open && !string.IsNullOrEmpty(notes.PageUrl)) OpenExternal(notes.PageUrl);
                    }
                    finally { EndNotice(); }
                }));
            }
            catch { }
        }

        async Task InitWeb()
        {
            try
            {
                status = L.T("Starting browser…", "Browser startet…");
                Invalidate(rFooter);

                string runtime = CoreWebView2Environment.GetAvailableBrowserVersionString();
                CoreWebView2Environment env = await CoreWebView2Environment.CreateAsync(
                    null, Store.ProfileDir, Edge.Options(settings));
                await web.EnsureCoreWebView2Async(env);
                core = web.CoreWebView2;

                core.Settings.AreDefaultContextMenusEnabled = false;
                core.Settings.AreDevToolsEnabled = true;
                core.Settings.IsStatusBarEnabled = false;
                core.Settings.IsZoomControlEnabled = false;
                core.Settings.IsSwipeNavigationEnabled = false;
                core.Settings.IsPinchZoomEnabled = false;
                core.Settings.AreBrowserAcceleratorKeysEnabled = false;
                core.Settings.IsPasswordAutosaveEnabled = true;
                core.Settings.IsGeneralAutofillEnabled = true;

                core.FrameCreated += OnFrameCreated;
                core.NavigationStarting += OnNavigationStarting;
                core.NavigationCompleted += OnNavigationCompleted;
                core.SourceChanged += delegate { Invalidate(rFooter); };
                core.NewWindowRequested += OnNewWindowRequested;
                core.ProcessFailed += OnProcessFailed;

                await core.AddScriptToExecuteOnDocumentCreatedAsync(FpsScript);

                web.ZoomFactor = settings.Zoom;
                fpsTimer.Start();
                StartPresence();
                roomTimer.Start();

                clock.Restart();
                core.Navigate(AppInfo.ClientUrl);
                status = L.T("Loading hotel…", "Hotel wird geladen…");
                Invalidate();
            }
            catch (WebView2RuntimeNotFoundException)
            {
                status = L.T("WebView2 runtime missing", "WebView2-Runtime fehlt");
                Invalidate();
                MessageSheetForm.Error(this, L.T("WebView2 is missing", "WebView2 fehlt"),
                    L.T("This client renders the hotel with the Microsoft Edge WebView2 runtime, which is not installed on this PC.",
                        "Dieser Client rendert das Hotel mit der Microsoft Edge WebView2-Runtime, die auf diesem PC fehlt."),
                    "https://developer.microsoft.com/microsoft-edge/webview2/");
            }
            catch (Exception ex)
            {
                status = L.T("Browser failed to start", "Browser konnte nicht starten");
                Invalidate();
                MessageSheetForm.Error(this, L.T("The browser could not start", "Der Browser konnte nicht starten"),
                    ex.Message, Store.ProfileDir);
            }
        }

        void OnNavigationStarting(object sender, CoreWebView2NavigationStartingEventArgs e)
        {
            loading = true;
            status = L.T("Loading hotel…", "Hotel wird geladen…");
            phase = -0.6f;
            anim.Start();
            Invalidate();
        }

        void OnNavigationCompleted(object sender, CoreWebView2NavigationCompletedEventArgs e)
        {
            loading = false;
            anim.Stop();
            lastNavOk = e.IsSuccess;
            lastNavStatus = e.WebErrorStatus;
            lastNavHttp = e.HttpStatusCode;
            lastNavUrl = core != null ? core.Source : "";
            lastNavMs = clock.ElapsedMilliseconds;

            if (e.IsSuccess)
            {
                status = L.T("Connected", "Verbunden");
                PollRoom();
                if (Program.ProbePath != null) probeTimer.Start();
                if (Program.SmokeLog != null && !smokeStarted)
                {
                    smokeStarted = true;
                    smokeTimer.Start();
                }
                if (Program.ShotLive != null && !liveStarted)
                {
                    liveStarted = true;
                    liveTimer.Start();
                }
            }
            else
            {
                status = L.T("Connection failed", "Verbindung fehlgeschlagen");
                if (e.WebErrorStatus != CoreWebView2WebErrorStatus.OperationCanceled &&
                    (DateTime.Now - lastErrorShown).TotalSeconds > 4)
                {
                    lastErrorShown = DateTime.Now;
                    MessageSheetForm.Error(this, L.T("The hotel did not load", "Das Hotel wurde nicht geladen"),
                        L.T("The page could not be reached. Check your connection and press Reload.",
                           "Die Seite war nicht erreichbar. Verbindung prüfen und neu laden."),
                        e.WebErrorStatus.ToString());
                }
            }
            Invalidate();
        }

        void OnNewWindowRequested(object sender, CoreWebView2NewWindowRequestedEventArgs e)
        {
            e.Handled = true;
            Uri u;
            if (Uri.TryCreate(e.Uri, UriKind.Absolute, out u) &&
                (u.Host.EndsWith("habbo.si", StringComparison.OrdinalIgnoreCase)))
            {
                if (core != null) core.Navigate(e.Uri);
                return;
            }
            OpenExternal(e.Uri);
        }

        void OpenExternal(string url)
        {
            try
            {
                Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
            }
            catch { }
        }

        void OnProcessFailed(object sender, CoreWebView2ProcessFailedEventArgs e)
        {
            if (e.ProcessFailedKind == CoreWebView2ProcessFailedKind.BrowserProcessExited)
            {
                status = L.T("Browser process stopped", "Browserprozess beendet");
                Invalidate();
            }
        }

        // ------------------------------------------------------------------ keyboard
        // The hotel's page owns the keyboard once it has focus, so a message filter is the
        // place to claim the client keys. It only answers while this window is the active one,
        // which keeps the shortcuts out of the settings sheet.
        public bool PreFilterMessage(ref Message m)
        {
            if (m.Msg != 0x100 && m.Msg != 0x104) return false;
            if (Form.ActiveForm != this) return false;

            Keys key = (Keys)(int)m.WParam;
            if (key == Keys.F5) { Reload(); return true; }
            if (key == Keys.F11) { ToggleFullscreen(); return true; }
            if (key == Keys.F12) { OpenDevTools(); return true; }
            if (key == Keys.Escape && fullscreen) { ToggleFullscreen(); return true; }
            return false;
        }

        async void PollFps()
        {
            if (core == null) { fps = -1; Invalidate(rFooter); return; }
            try
            {
                string raw = await core.ExecuteScriptAsync(
                    "(window.__hsc && typeof window.__hsc.fps === 'number') ? window.__hsc.fps : -1");
                int v;
                if (int.TryParse(raw.Trim('"'), NumberStyles.Integer, CultureInfo.InvariantCulture, out v)) fps = v;
                else fps = -1;
            }
            catch { fps = -1; }
            Invalidate(rFooter);
        }

        // ------------------------------------------------------------------ actions
        void Reload()
        {
            if (core == null) return;
            core.Reload();
        }

        void OpenDevTools()
        {
            if (core == null || !settings.DevTools) return;
            try { core.OpenDevToolsWindow(); } catch { }
        }

        void ZoomBy(float delta)
        {
            float next = settings.Zoom + delta;
            if (next < 0.5f) next = 0.5f;
            if (next > 2f) next = 2f;
            settings.Zoom = (float)Math.Round(next, 2);
            if (web != null) web.ZoomFactor = settings.Zoom;
            toast = L.T("Zoom ", "Zoom ") + Math.Round(settings.Zoom * 100) + "%";
            toastTimer.Stop();
            toastTimer.Start();
            Invalidate();
        }

        void OpenSettings()
        {
            using (SettingsSheet sheet = new SettingsSheet(settings, core))
            {
                DialogResult result = sheet.ShowDialog(this);
                if (sheet.ReleaseNotesRequested)
                {
                    ShowReleaseNotes();
                    return;
                }
                if (result != DialogResult.OK) return;
                bool restart = sheet.BrowserArgsChanged;
                bool discord = sheet.DiscordChanged;
                bool channelChanged = sheet.Result.UpdateChannel != settings.UpdateChannel;
                settings = sheet.Result;
                L.De = settings.German;
                ApplyChrome();
                LayoutAll();
                Invalidate(true);
                if (discord) StartPresence();
                if (channelChanged) CheckForUpdates();
                if (restart) ShowToast(L.T("Restart the client to apply graphics settings",
                    "Client neu starten, um Grafik-Einstellungen anzuwenden"));
            }
        }

        void ApplyChrome()
        {
            T.Apply(settings.Mode);
            backColor(T.Bg1, this);
            tips.SetToolTip(chipReload, ReloadTip());
            tips.SetToolTip(chipSettings, SettingsTip());
            tips.SetToolTip(pill, ClientTip());
#if BETA
            tips.SetToolTip(betaPill, L.T("Beta channel build", "Beta-Kanal-Build"));
#endif
            if (web != null) web.DefaultBackgroundColor = T.Bg1;
            if (web != null) web.ZoomFactor = settings.Zoom;
        }

        static void backColor(Color c, Control root)
        {
            foreach (Control child in root.Controls) child.BackColor = c;
            root.BackColor = c;
        }

        static string ReloadTip()
        {
            return L.T("Reload (F5)", "Neu laden (F5)");
        }

        static string SettingsTip()
        {
            return L.T("Settings", "Einstellungen");
        }

        static string ClientTip()
        {
            return AppInfo.Name + " v" + AppInfo.Version + "\n" +
                   L.T("Build date", "Build-Datum") + " " + AppInfo.BuildDate;
        }

        void ShowToast(string text)
        {
            toast = text;
            toastTimer.Stop();
            toastTimer.Start();
            Invalidate(rFooter);
        }

        void ToggleMaximize()
        {
            if (WindowState == FormWindowState.Maximized) WindowState = FormWindowState.Normal;
            else WindowState = FormWindowState.Maximized;
            wbMax.Icon = WindowState == FormWindowState.Maximized ? "restore" : "max";
            wbMax.Invalidate();
            LayoutAll();
        }

        void ToggleFullscreen()
        {
            fullscreen = !fullscreen;
            if (fullscreen)
            {
                prevState = WindowState;
                prevBounds = Bounds;
                WindowState = FormWindowState.Normal;
                Bounds = Screen.FromControl(this).Bounds;
                WindowState = FormWindowState.Maximized;
            }
            else
            {
                WindowState = prevState;
                if (prevState == FormWindowState.Normal) Bounds = prevBounds;
            }
            LayoutAll();
            wbMin.Visible = wbMax.Visible = wbClose.Visible = !fullscreen;
            chipReload.Invalidate();
            chipSettings.Invalidate();
            Invalidate(true);
        }

        protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
        {
            if (keyData == Keys.F5) { Reload(); return true; }
            if (keyData == Keys.F11) { ToggleFullscreen(); return true; }
            if (keyData == Keys.F12) { OpenDevTools(); return true; }
            if (keyData == Keys.Escape && fullscreen) { ToggleFullscreen(); return true; }
            if (keyData == (Keys.Control | Keys.D0)) { settings.Zoom = 1f; if (web != null) web.ZoomFactor = 1f; Invalidate(); return true; }
            if (keyData == (Keys.Control | Keys.Oemplus)) { ZoomBy(0.1f); return true; }
            if (keyData == (Keys.Control | Keys.OemMinus)) { ZoomBy(-0.1f); return true; }
            return base.ProcessCmdKey(ref msg, keyData);
        }

        // ------------------------------------------------------------------ smoke test
        async void FinishSmoke()
        {
            if (Program.SmokeLog == null) return;
            StringBuilder sb = new StringBuilder();
            try
            {
                string runtime = CoreWebView2Environment.GetAvailableBrowserVersionString();
                string title = "";
                string fpsText = "";
                try
                {
                    title = await core.ExecuteScriptAsync("document.title");
                    fpsText = await core.ExecuteScriptAsync("String(window.__hsc ? window.__hsc.fps : -1)");
                }
                catch { }

                sb.AppendLine("runtime=" + runtime);
                sb.AppendLine("nav_success=" + lastNavOk);
                sb.AppendLine("nav_status=" + lastNavStatus);
                sb.AppendLine("http=" + lastNavHttp);
                sb.AppendLine("url=" + lastNavUrl);
                sb.AppendLine("elapsed_ms=" + lastNavMs);
                sb.AppendLine("title=" + title.Trim().Trim('"'));
                sb.AppendLine("fps=" + fpsText.Trim().Trim('"'));
                sb.AppendLine("webview2=" + "ok");
                File.WriteAllText(Program.SmokeLog, sb.ToString());

                if (Program.SmokeImage != null)
                {
                    using (FileStream fs = new FileStream(Program.SmokeImage, FileMode.Create, FileAccess.Write))
                        await core.CapturePreviewAsync(CoreWebView2CapturePreviewImageFormat.Png, fs);
                }
            }
            catch (Exception ex)
            {
                try { File.WriteAllText(Program.SmokeLog, "error=" + ex.Message); } catch { }
            }
            Close();
        }

        // ------------------------------------------------------------------ capture
        /// <summary>
        /// Captures the whole window, chrome and hotel together. WebView2 paints through
        /// DirectComposition, so this asks DWM to render the window rather than using
        /// DrawToBitmap; used by the documentation build only.
        /// </summary>
        void CaptureWindow(string path)
        {
            if (path == null) return;
            try
            {
                using (Bitmap bmp = new Bitmap(Width, Height))
                {
                    using (Graphics g = Graphics.FromImage(bmp))
                    {
                        IntPtr hdc = g.GetHdc();
                        PrintWindow(Handle, hdc, 2);   // PW_RENDERFULLCONTENT
                        g.ReleaseHdc(hdc);
                    }
                    bmp.Save(path, System.Drawing.Imaging.ImageFormat.Png);
                }
            }
            catch { }
        }

        // ------------------------------------------------------------------ chrome
        protected override void OnResize(EventArgs e)
        {
            base.OnResize(e);
            LoadLayout();
        }

        void LoadLayout()
        {
            LayoutAll();
            Invalidate();
        }

        protected override void OnMouseDown(MouseEventArgs e)
        {
            base.OnMouseDown(e);
            if (e.Button != MouseButtons.Left || fullscreen) return;
            if (e.Y < rBar.Bottom)
            {
                ReleaseCapture();
                SendMessage(Handle, 0xA1, (IntPtr)0x2, IntPtr.Zero);
            }
        }

        protected override void OnMouseDoubleClick(MouseEventArgs e)
        {
            base.OnMouseDoubleClick(e);
            if (!fullscreen && e.Y < rBar.Bottom) ToggleMaximize();
        }

        const int WM_NCHITTEST = 0x84;
        const int HTCLIENT = 1, HTLEFT = 10, HTRIGHT = 11, HTTOP = 12, HTTOPLEFT = 13, HTTOPRIGHT = 14,
                  HTBOTTOM = 15, HTBOTTOMLEFT = 16, HTBOTTOMRIGHT = 17;

        protected override void WndProc(ref Message m)
        {
            if (m.Msg == WM_NCHITTEST)
            {
                base.WndProc(ref m);
                if ((int)m.Result == HTCLIENT && WindowState == FormWindowState.Normal)
                {
                    int lp = m.LParam.ToInt32();
                    int sx = (short)(lp & 0xFFFF);
                    int sy = (short)((lp >> 16) & 0xFFFF);
                    Point p = PointToClient(new Point(sx, sy));
                    int g = T.P(7);
                    bool left = p.X <= g, right = p.X >= ClientSize.Width - g;
                    bool top = p.Y <= g, bottom = p.Y >= ClientSize.Height - g;
                    if (top && left) m.Result = (IntPtr)HTTOPLEFT;
                    else if (top && right) m.Result = (IntPtr)HTTOPRIGHT;
                    else if (bottom && left) m.Result = (IntPtr)HTBOTTOMLEFT;
                    else if (bottom && right) m.Result = (IntPtr)HTBOTTOMRIGHT;
                    else if (left) m.Result = (IntPtr)HTLEFT;
                    else if (right) m.Result = (IntPtr)HTRIGHT;
                    else if (top) m.Result = (IntPtr)HTTOP;
                    else if (bottom) m.Result = (IntPtr)HTBOTTOM;
                }
                return;
            }
            base.WndProc(ref m);
        }

        [DllImport("user32.dll")]
        static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);

        [DllImport("user32.dll")]
        static extern bool ReleaseCapture();

        [DllImport("user32.dll")]
        static extern IntPtr SendMessage(IntPtr hWnd, int msg, IntPtr wParam, IntPtr lParam);

        // ------------------------------------------------------------- Discord presence

        const string RoomProbe = @"(function(){try{
function clean(s){if(!s)return '';s=String(s).replace(/\s+/g,' ').trim();if(s.length<2||s.length>90)return '';return s;}
function generic(n){n=(n||'').toLowerCase().replace(/\s+/g,' ').trim();
return n===''||n==='habbo'||n==='habbo.si'||n==='einchecken'||n==='login'||n==='willkommen'||n==='registrieren'||n==='hotel'||n==='client'||n==='room info'||n==='rauminfo'||n==='info'||n==='settings'||n==='navigator'||n==='catalog'||n==='inventory'||n==='menu'||n==='owner'||n==='besitzer'||n==='description'||n==='beschreibung';}
function ok(s){var v=clean(s);return v&&!generic(v)?v:'';}
function peekProps(o){
if(!o||typeof o!=='object')return '';
try{
var ss=o.roomSession;
if(ss){var v=ok(ss.roomName);if(v)return v;if(ss.roomData)return ok(ss.roomData.name);}
}catch(e1){}
return '';
}
function peekState(o){
if(!o||typeof o!=='object')return '';
try{
var v=ok(o.roomName);if(v)return v;
if(o.roomData)return ok(o.roomData.name);
}catch(e1){}
return '';
}
function fromFiber(){
var el=document.getElementById('root')||document.body;if(!el)return '';
var k=Object.keys(el).filter(function(x){return x.indexOf('__reactContainer$')===0;})[0];
var f=k?el[k]:(el._reactRootContainer?el._reactRootContainer._internalRoot.current:null);
if(!f)return '';
var stack=[f],seen=0;
while(stack.length&&seen<9000){
var n=stack.pop();seen++;
var v=peekProps(n.memoizedProps);
if(!v&&n.memoizedState){var h=n.memoizedState,c=0;while(h&&c<24){v=peekState(h.memoizedState);if(v)break;h=h.next;c++;}}
if(v)return v;
if(n.sibling)stack.push(n.sibling);
if(n.child)stack.push(n.child);
}
return '';
}
var room='';
var sels=['[data-room-name]','[data-roomname]','.room-name','.roomName','[class*=""room-name"" i]','[class*=""roomName"" i]','[class*=""nitro-room-tools-info"" i]','[class*=""room-tools-info"" i]','[class*=""room-info"" i]','[class*=""roominfo"" i]','[id*=""room-info"" i]'];
for(var i=0;i<sels.length&&!room;i++){
var els=document.querySelectorAll(sels[i]);
for(var j=0;j<els.length&&j<8;j++){
var el=els[j],a=ok(el.getAttribute&&el.getAttribute('data-room-name'));
if(a){room=a;break;}
var txt=(el.innerText||'').split('\n');
for(var q=0;q<txt.length;q++){var t=ok(txt[q]);if(t){room=t;break;}}
if(room)break;
}}
var fiber='';
try{if(!room){var g=window.Nitro||window.nitro;if(g){var ins=g.instance||g;var ss=ins.roomSession||(ins.roomSessionManager&&ins.roomSessionManager.getSession?ins.roomSessionManager.getSession(-1):null);if(ss)room=ok(ss.roomName||(ss.roomData&&ss.roomData.name)||'');}}}catch(e2){}
try{if(!room&&window.GetNitroInstance){var ins2=window.GetNitroInstance();var ss2=ins2&&ins2.roomSessionManager&&ins2.roomSessionManager.getSession?ins2.roomSessionManager.getSession(-1):null;if(ss2)room=ok(ss2.roomName||(ss2.roomData&&ss2.roomData.name)||'');}}catch(e3){}
var title=clean(document.title),titleRoom='';
var m=title.match(/^(.*?)\s*(?:[-–—|·]\s*)(?:Habbo|Habbo\.si|Hotel|Nitro)(?:\s.*)?$/i);
if(m&&m[1])titleRoom=ok(m[1]);
// The cheap DOM and title checks answer for nearly every room; walking the React
// fiber tree is only the fallback, so a poll no longer costs a full tree scan.
if(!room)fiber=fromFiber();
return JSON.stringify({room:room,fiber:fiber,title:title,titleRoom:titleRoom});
}catch(e){return '';}})();";

        void StartPresence()
        {
            if (!presenceAllowed) return;
            if (!settings.Discord) { DiscordRpc.Stop(); return; }
            string id = DiscordRpc.ResolveAppId(settings.DiscordAppId);
            if (!DiscordRpc.IsValidAppId(id)) { DiscordRpc.Stop(); return; }
            DiscordRpc.Start(id, DiscordRpc.ArtworkUrl);
            UpdatePresence();
        }

        internal const string PresenceState = "On habbo.si";

        void UpdatePresence()
        {
            DiscordRpc.SetActivity(PresenceDetails(currentRoom), PresenceState);
        }

        /// <summary>
        /// The activity lines Discord shows. Sentence case throughout; the room keeps the hotel's
        /// own spelling unless it arrives shouty or all lowercase, in which case it is title-cased
        /// so the line never reads like a broken label. The state stays "On habbo.si" because the
        /// domain is branded lowercase.
        /// </summary>
        internal static string PresenceDetails(string room)
        {
            room = PrettyRoom(room);
            return room.Length == 0 ? "Exploring the hotel" : "Hanging out in " + room;
        }

        static string PrettyRoom(string room)
        {
            if (string.IsNullOrEmpty(room)) return "";
            string s = Regex.Replace(room.Trim(), @"\s+", " ");
            if (s.Length > 48) s = s.Substring(0, 47).TrimEnd() + "…";

            string lower = s.ToLowerInvariant();
            bool shouty = s == s.ToUpperInvariant() && s != lower;
            bool flat = s == lower;
            if (shouty || flat) s = CultureInfo.CurrentCulture.TextInfo.ToTitleCase(lower);
            return s;
        }

        void OnFrameCreated(object sender, CoreWebView2FrameCreatedEventArgs e)
        {
            CoreWebView2Frame frame = e.Frame;
            frames.Add(frame);
            frame.Destroyed += delegate { frames.Remove(frame); };
        }

        async System.Threading.Tasks.Task<string> DetectRoom()
        {
            string room = null;
            if (core != null)
            {
                try { room = PickRoom(Unquote(await core.ExecuteScriptAsync(RoomProbe))); }
                catch { }
            }
            if (string.IsNullOrEmpty(room))
            {
                CoreWebView2Frame[] list = frames.ToArray();
                for (int i = 0; i < list.Length && string.IsNullOrEmpty(room); i++)
                {
                    try { room = PickRoom(Unquote(await list[i].ExecuteScriptAsync(RoomProbe))); }
                    catch { }
                }
            }
            return room ?? "";
        }

        async void PollRoom()
        {
            if (!presenceAllowed || core == null || !settings.Discord) return;
            string room = await DetectRoom();
            if (room == currentRoom) return;
            currentRoom = room;
            UpdatePresence();
        }

        static string PickRoom(string json)
        {
            if (string.IsNullOrEmpty(json)) return null;
            string room = JsonField(json, "room");
            if (!string.IsNullOrEmpty(room) && !IsGeneric(room)) return room;
            string fiber = JsonField(json, "fiber");
            if (!string.IsNullOrEmpty(fiber) && !IsGeneric(fiber)) return fiber;
            string fromTitle = JsonField(json, "titleRoom");
            if (!string.IsNullOrEmpty(fromTitle) && !IsGeneric(fromTitle)) return fromTitle;
            return null;
        }

        static bool IsGeneric(string name)
        {
            string n = name.Trim().ToLowerInvariant();
            return n == "habbo" || n == "habbo.si" || n == "einchecken" || n == "login" ||
                   n == "willkommen" || n == "registrieren" || n == "hotel" || n == "client" ||
                   n == "room info" || n == "rauminfo" || n == "info" || n == "settings" ||
                   n == "navigator" || n == "catalog" || n == "inventory" || n == "menu";
        }

        static string JsonField(string json, string key)
        {
            string needle = "\"" + key + "\":\"";
            int i = json.IndexOf(needle, StringComparison.Ordinal);
            if (i < 0) return "";
            i += needle.Length;
            StringBuilder sb = new StringBuilder();
            for (; i < json.Length; i++)
            {
                char c = json[i];
                if (c == '\\' && i + 1 < json.Length) { sb.Append(json[++i]); continue; }
                if (c == '"') break;
                sb.Append(c);
            }
            return sb.ToString().Trim();
        }

        /// <summary>ExecuteScriptAsync returns a JSON string; unwrap it before parsing.</summary>
        static string Unquote(string raw)
        {
            if (string.IsNullOrEmpty(raw)) return "";
            if (raw.Length < 2 || raw[0] != '"' || raw[raw.Length - 1] != '"') return raw;
            StringBuilder sb = new StringBuilder();
            for (int i = 1; i < raw.Length - 1; i++)
            {
                char c = raw[i];
                if (c != '\\' || i + 1 >= raw.Length - 1) { sb.Append(c); continue; }
                char n = raw[++i];
                if (n == 'n') sb.Append('\n');
                else if (n == 'r') sb.Append('\r');
                else if (n == 't') sb.Append('\t');
                else if (n == 'u' && i + 4 < raw.Length)
                {
                    int code;
                    if (int.TryParse(raw.Substring(i + 1, 4), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out code))
                    {
                        sb.Append((char)code);
                        i += 4;
                        continue;
                    }
                    sb.Append(n);
                }
                else sb.Append(n);
            }
            return sb.ToString();
        }

        // ------------------------------------------------------------- diagnostics

        const string DiagScript = @"(function(){try{
function cls(el){try{return (el.className&&el.className.toString)?el.className.toString():'';}catch(e){return '';}}
var out={url:location.href,title:document.title,ready:document.readyState,keys:[],nodes:[]};
try{out.keys=Object.keys(window).filter(function(k){return /nitro|room|habbo|client|user|engine/i.test(k);}).slice(0,50);}catch(e){}
var all=document.querySelectorAll('*');
for(var i=0;i<all.length&&out.nodes.length<70;i++){
var el=all[i],c=cls(el),id=el.id||'';
if(!/room|user|name|info|header/i.test(c+' '+id))continue;
var t=(el.innerText||'').replace(/\s+/g,' ').trim();
if(!t||t.length>140)continue;
out.nodes.push({t:el.tagName,c:c.slice(0,100),i:id,x:t,n:el.children.length});
}
out.text=(document.body.innerText||'').replace(/\s+/g,' ').slice(0,1800);
return JSON.stringify(out);
}catch(e){return JSON.stringify({err:String(e)});}})();";

        async void RunProbe()
        {
            probeTimer.Stop();
            string path = Program.ProbePath;
            if (path == null) return;
            string probe = "", diag = "";
            try { probe = Unquote(await core.ExecuteScriptAsync(RoomProbe)); }
            catch (Exception ex) { probe = "error: " + ex.Message; }
            try { diag = Unquote(await core.ExecuteScriptAsync(DiagScript)); }
            catch (Exception ex) { diag = "error: " + ex.Message; }
            string detected = "";
            try { detected = await DetectRoom(); } catch (Exception ex) { detected = "error: " + ex.Message; }
            if (detected.Length == 0)
            {
                await System.Threading.Tasks.Task.Delay(3500);
                try { detected = await DetectRoom(); } catch { }
            }

            StringBuilder sb = new StringBuilder();
            sb.AppendLine("detected=" + detected);
            sb.AppendLine("room_probe=" + probe);
            sb.AppendLine("dom=" + diag);
            sb.AppendLine("frames=" + frames.Count);
            foreach (CoreWebView2Frame f in frames)
            {
                try { sb.AppendLine("frame " + f.FrameId + " name='" + f.Name + "' " + Unquote(await f.ExecuteScriptAsync(DiagScript))); }
                catch (Exception ex) { sb.AppendLine("frame " + f.FrameId + " error=" + ex.Message); }
            }
            try { sb.AppendLine("frame_tree=" + await core.CallDevToolsProtocolMethodAsync("Page.getFrameTree", "{}")); }
            catch (Exception ex) { sb.AppendLine("frame_tree_error=" + ex.Message); }
            try { File.WriteAllText(path, sb.ToString()); }
            catch { }
            Close();
        }

        protected override void OnFormClosing(FormClosingEventArgs e)
        {
            Application.RemoveMessageFilter(this);
            bool previewRun = Program.ShotPath != null || Program.SmokeLog != null || Program.ShotLive != null;
            settings.Maximized = WindowState == FormWindowState.Maximized;
            if (WindowState == FormWindowState.Normal) { settings.Width = Width; settings.Height = Height; }
            if (!previewRun) Store.Save(settings);

            anim.Stop();
            fpsTimer.Stop();
            toastTimer.Stop();
            smokeTimer.Stop();
            liveTimer.Stop();
            roomTimer.Stop();
            probeTimer.Stop();
            tips.Dispose();
            DiscordRpc.Stop();
            try { if (web != null) web.Dispose(); } catch { }
            base.OnFormClosing(e);
        }
    }
}
