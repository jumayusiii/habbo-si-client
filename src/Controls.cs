using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Windows.Forms;

namespace HabboSiClient
{
    /// <summary>Fades a borderless sheet in and out, and keeps its DialogResult intact.</summary>
    internal static class SheetFade
    {
        const int InMs = 150, OutMs = 120;

        public static void Attach(Form sheet)
        {
            // "Show animations in Windows" is off: no fade, no layered window, just the sheet.
            if (!SystemInformation.UIEffectsEnabled) return;

            Timer timer = new Timer();
            System.Diagnostics.Stopwatch clock = new System.Diagnostics.Stopwatch();
            bool closing = false;
            bool finished = false;
            int span = InMs;
            DialogResult wanted = DialogResult.None;
            float from = 0f, to = 1f;

            sheet.Opacity = 0;

            sheet.Shown += delegate
            {
                if (closing) return;
                from = (float)sheet.Opacity;
                to = 1f;
                span = InMs;
                clock.Reset();
                clock.Start();
                timer.Start();
            };

            sheet.FormClosing += delegate(object sender, FormClosingEventArgs e)
            {
                if (finished) return;
                if (closing) { e.Cancel = true; return; }

                if (e.CloseReason != CloseReason.None && e.CloseReason != CloseReason.UserClosing) return;
                if (!sheet.IsHandleCreated || sheet.WindowState == FormWindowState.Minimized) return;

                wanted = sheet.DialogResult;
                e.Cancel = true;
                closing = true;
                from = (float)sheet.Opacity;
                to = 0f;
                span = OutMs;
                clock.Reset();
                clock.Start();
                timer.Start();
            };

            timer.Interval = 15;
            timer.Tick += delegate
            {
                if (sheet.IsDisposed || !sheet.IsHandleCreated) { timer.Stop(); return; }

                float t = span <= 0 ? 1f : Math.Min(1f, (float)clock.ElapsedMilliseconds / span);
                float eased = t * t * (3f - 2f * t);

                if (closing)
                {
                    if (t >= 1f)
                    {
                        timer.Stop();
                        finished = true;
                        if (wanted != DialogResult.None && sheet.DialogResult != wanted) sheet.DialogResult = wanted;
                        sheet.Close();
                        return;
                    }
                    sheet.Opacity = from + (to - from) * eased;
                    return;
                }

                sheet.Opacity = from + (to - from) * eased;
                if (t >= 1f) timer.Stop();
            };

            sheet.FormClosed += delegate
            {
                timer.Stop();
                timer.Dispose();
            };
        }
    }

    /// <summary>Rounded button with an optional leading icon.</summary>
    internal class FlatBtn : Control
    {
        public string Label = "";
        public string Icon = null;
        public bool Primary = false;
        public bool Danger = false;
        bool hot, down;

        public FlatBtn()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Card;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            RectangleF r = new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f);
            float rad = T.F(8f);
            Color fg = Danger ? T.DangerButtonFace(g, r, rad, hot, down)
                              : T.ButtonFace(g, r, rad, Primary, hot, down);

            float iw = Icon != null ? T.F(15f) : 0f;
            float gap = Icon != null ? T.F(7f) : 0f;
            Size ts = Label.Length > 0 ? T.Meas(Label, T.FBtn) : new Size(0, 0);
            float total = iw + gap + ts.Width;
            float x = (Width - total) / 2f;
            if (Icon != null)
                T.Icon(g, Icon, new RectangleF(x, (Height - iw) / 2f - 0.5f, iw, iw), fg);
            if (Label.Length > 0)
            {
                Rectangle tr = new Rectangle((int)Math.Round(x + iw + gap), 0, (int)ts.Width + 3, Height);
                T.Str(g, Label, T.FBtn, fg, tr, T.FL);
            }
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; down = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { down = true; Invalidate(); base.OnMouseDown(e); }
        protected override void OnMouseUp(MouseEventArgs e) { down = false; Invalidate(); base.OnMouseUp(e); }
    }

    /// <summary>Square icon button, optionally a toggle.</summary>
    internal class IconBtn : Control
    {
        public string Icon = "gear";
        public bool Active = false;
        public bool Segmented = false;
        public float IconScale = 0.56f;
        bool hot, down;

        public IconBtn()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Bg1;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            RectangleF r = new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f);
            float rad = T.F(8f);
            Color fg;
            if (Active) fg = T.ButtonFace(g, r, rad, true, hot, down);
            else if (Segmented)
            {
                if (hot) T.FillRound(g, r, rad, T.HoverSoft);
                fg = hot ? T.Text : T.IconIdle;
            }
            else fg = T.ButtonFace(g, r, rad, false, hot, down);
            float s = Math.Min(Width, Height) * IconScale;
            T.Icon(g, Icon, new RectangleF((Width - s) / 2f, (Height - s) / 2f, s, s), fg);
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; down = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { down = true; Invalidate(); base.OnMouseDown(e); }
        protected override void OnMouseUp(MouseEventArgs e) { down = false; Invalidate(); base.OnMouseUp(e); }
    }

    /// <summary>Caption button: minimise, maximise/restore and close.</summary>
    internal class WinBtn : Control
    {
        public string Icon = "min";
        bool hot;

        public WinBtn()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Bg1;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush b = new SolidBrush(BackColor)) g.FillRectangle(b, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            bool close = Icon == "close";
            if (hot)
                T.FillRound(g, new RectangleF(T.P(3), T.P(2), Width - T.P(6), Height - T.P(4)),
                    T.F(7f), close ? Color.FromArgb(216, 62, 68) : T.Hover);
            Color fg = hot ? Color.White : T.WinFg;
            float s = T.F(14f);
            T.Icon(g, Icon, new RectangleF((Width - s) / 2f, (Height - s) / 2f, s, s), fg);
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
    }

    /// <summary>App bar chip with an icon, a label and an optional status dot.</summary>
    internal class LinkChip : Control
    {
        public string Label = "Settings";
        public string Icon = "gear";
        int status;
        public int Status   // 0 hidden, 1 waiting, 2 connected, 3 error
        {
            get { return status; }
            set
            {
                if (status == value) return;
                status = value;
                Invalidate();
            }
        }
        bool hot;

        public LinkChip()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Bg1;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            T.FillRound(g, new RectangleF(0f, 0f, Width, Height), T.F(7f), hot ? T.HoverSoft : T.Segment);

            // A chip without a label is the icon-only variant: centre the glyph and skip the text.
            bool iconOnly = Label.Length == 0;
            float ic = iconOnly ? T.F(13f) : T.F(11f);
            float ix = iconOnly ? (Width - ic) / 2f : T.P(10);
            T.Icon(g, Icon, new RectangleF(ix, (Height - ic) / 2f, ic, ic), hot ? T.GoldHi : T.Gold);
            if (!iconOnly)
            {
                int tx = (int)Math.Round(ix + ic) + T.P(6);
                int dotW = Status > 0 ? T.P(15) : 0;
                T.Str(g, Label, T.FChip, hot ? T.Text : T.ChipText,
                    new Rectangle(tx, 0, Width - tx - T.P(7) - dotW, Height), T.FL);
            }

            if (Status > 0)
            {
                Color dc = Status == 2 ? T.Teal : Status == 3 ? T.Danger : T.Gold;
                float d = T.F(6f);
                using (SolidBrush b = new SolidBrush(dc))
                    g.FillEllipse(b, Width - T.P(11) - d, (Height - d) / 2f, d, d);
            }
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
    }

    /// <summary>Small outlined gold pill: the client tag next to the wordmark.</summary>
    internal class TagPill : Control
    {
        public string Caption = "CLIENT";
        bool hot, interactive;

        /// <summary>Header pills light up and take the hand cursor; plain tags stay flat.</summary>
        public bool Interactive
        {
            get { return interactive; }
            set { interactive = value; Cursor = value ? Cursors.Hand : Cursors.Default; }
        }

        public TagPill()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Bg1;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            g.SmoothingMode = SmoothingMode.AntiAlias;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);

            // A capsule that hugs its label: the control is sized from the text in LayoutAll.
            RectangleF r = new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f);
            float radius = Height / 2f;
            bool live = interactive && hot;
            T.FillRound(g, r, radius, Color.FromArgb(live ? 44 : 26, T.Gold));
            T.StrokeRound(g, r, radius, live ? T.GoldHi : T.Gold, live ? 1.4f : 1f);
            T.Str(g, Caption, T.FLabel, live ? T.GoldHi : T.Gold, ClientRectangle, T.FC);
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
    }

    /// <summary>EN | DE two-way language switch.</summary>
    internal class LangSwitch : Control
    {
        public bool German;
        bool hot;
        public event Action Changed;

        public LangSwitch()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Bg1;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            T.FillRound(g, new RectangleF(0, 0, Width, Height), T.F(7f), T.Segment);
            int half = Width / 2;
            RectangleF act = German
                ? new RectangleF(half, 1, half - 1, Height - 2)
                : new RectangleF(1, 1, half - 1, Height - 2);
            T.FillRound(g, act, T.F(6f), T.LangPill);
            Color on = T.LangPillText;
            Color off = hot ? T.Text : T.Muted;
            T.Str(g, "EN", T.FChip, German ? off : on, new Rectangle(0, 0, half, Height), T.FC);
            T.Str(g, "DE", T.FChip, German ? on : off, new Rectangle(half, 0, Width - half, Height), T.FC);
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e)
        {
            base.OnMouseDown(e);
            German = !German;
            Invalidate();
            if (Changed != null) Changed();
        }
    }

    /// <summary>Appearance picker tile: a miniature of the client window in that palette.</summary>
    internal class ThemeTile : Control
    {
        public ThemeMode Mode = ThemeMode.Dark;
        public bool Selected;
        bool hot;

        public ThemeTile()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Card;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            RectangleF r = new RectangleF(1f, 1f, Width - 2f, Height - 2f);
            T.FillRound(g, r, T.F(10f), T.MiniBg(Mode));

            int pad = T.P(9);
            RectangleF win = new RectangleF(pad, pad, Width - pad * 2, Height - pad * 2 - T.P(24));
            T.FillRound(g, win, T.F(6f), T.MiniBg(Mode));
            T.StrokeRound(g, win, T.F(6f), T.MiniStroke(Mode), 1f);

            T.FillRound(g, new RectangleF(win.X + T.P(6), win.Y + T.P(6), T.P(20), T.P(5)), T.F(2f), T.MiniGold(Mode));
            using (SolidBrush d = new SolidBrush(T.MiniStroke(Mode)))
                for (int i = 0; i < 3; i++)
                    g.FillEllipse(d, win.Right - T.P(26) + i * T.P(7), win.Y + T.P(7), T.P(3), T.P(3));

            float bodyY = win.Y + T.P(16);
            float bodyH = win.Bottom - bodyY - T.P(6);
            RectangleF stage = new RectangleF(win.X + T.P(5), bodyY, win.Width - T.P(10), bodyH);
            using (SolidBrush cb = new SolidBrush(T.MiniCard(Mode)))
            using (GraphicsPath p = T.Round(stage, T.F(3f)))
                g.FillPath(cb, p);

            using (SolidBrush b = new SolidBrush(T.MiniGold(Mode)))
                g.FillRectangle(b, stage.X + T.P(5), stage.Y + bodyH * 0.62f, stage.Width - T.P(10), T.P(3));
            using (SolidBrush b = new SolidBrush(T.MiniStroke(Mode)))
            {
                g.FillRectangle(b, stage.X + T.P(5), stage.Y + T.P(5), stage.Width - T.P(10), T.P(3));
                g.FillRectangle(b, stage.X + T.P(5), stage.Y + T.P(12), stage.Width * 0.55f, T.P(3));
            }

            int ly = Height - T.P(22);
            Color lc = Selected ? T.Gold : (hot ? T.Text : T.Muted);
            string label = Mode == ThemeMode.Light ? L.T("Light", "Hell")
                : Mode == ThemeMode.Navy ? L.T("Navy", "Marineblau")
                : L.T("Dark", "Dunkel");
            float ic = T.F(12f);
            string icon = Mode == ThemeMode.Light ? "sun" : "moon";
            T.Icon(g, icon, new RectangleF(T.P(11), ly + (T.P(16) - ic) / 2f, ic, ic), lc);
            int lx = T.P(11) + (int)ic + T.P(7);
            T.StrEll(g, label, T.FBtn, lc, new Rectangle(lx, ly, Math.Max(T.P(34), Width - lx - T.P(26)), T.P(16)), T.FL);
            if (Selected)
            {
                float cs = T.F(12f);
                T.Icon(g, "check", new RectangleF(Width - T.P(11) - cs, ly + (T.P(16) - cs) / 2f, cs, cs), T.Gold);
            }

            T.StrokeRound(g, new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f), T.F(10f),
                Selected ? T.Gold : (hot ? T.StrokeHot : T.Stroke), Selected ? 2f : 1f);
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
    }

    /// <summary>A settings row with a label, a note and a switch; click anywhere on the row.</summary>
    internal class ToggleRow : Control
    {
        public string Label = "";
        public string Note = "";
        public bool On;
        public bool Restart;
        public event Action Changed;
        bool hot;

        public ToggleRow()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Card;
            Cursor = Cursors.Hand;
            TabStop = false;
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;

            if (hot) T.FillRound(g, new RectangleF(0, 1, Width, Height - 2), T.F(8f), T.HoverSoft);

            int pad = T.P(10);
            int textW = Width - pad * 2 - T.P(96);
            T.StrEll(g, Label, T.FBtn, T.Text, new Rectangle(pad, T.P(8), textW, T.P(17)), T.FL);
            if (Note.Length > 0)
                T.StrEll(g, Note, T.FMicro, T.Dim, new Rectangle(pad, T.P(25), textW, T.P(15)), T.FL);

            // switch
            float tw = T.P(38), th = T.P(20);
            RectangleF track = new RectangleF(Width - pad - tw, (Height - th) / 2f, tw, th);
            T.FillRound(g, track, th / 2f, On ? T.Gold : T.Stroke);
            float knob = th - T.P(4);
            float kx = On ? track.Right - knob - T.P(2) : track.X + T.P(2);
            using (SolidBrush b = new SolidBrush(On ? T.GoldText : T.Muted))
                g.FillEllipse(b, kx, track.Y + T.P(2), knob, knob);

            if (Restart)
            {
                int cw = T.Meas("RESTART", T.FMicro).Width + T.P(8);
                Rectangle chip = new Rectangle((int)track.X - T.P(8) - cw, (Height - T.P(14)) / 2, cw, T.P(14));
                T.FillRound(g, chip, T.F(7f), T.Gold14);
                T.Str(g, "RESTART", T.FMicro, T.Gold, chip, T.FC);
            }
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e)
        {
            base.OnMouseDown(e);
            On = !On;
            Invalidate();
            if (Changed != null) Changed();
        }
    }
}
