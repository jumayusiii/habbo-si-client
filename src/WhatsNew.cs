using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace HabboSiClient
{
    /// <summary>
    /// Themed "What's new" sheet: the GitHub notes of the release this build belongs to,
    /// shown once per profile after an update and reopenable from Settings.
    /// </summary>
    internal class WhatsNewForm : Form
    {
        readonly ReleaseNotes notes;
        TextBox text;
        FlatBtn openBtn, closeBtn;
        WinBtn close;
        Rectangle rHead, rFoot, rText;
        bool dragging;
        Point dragAt;

        /// <summary>True only when the user asked to open the release page.</summary>
        public static bool ShowSheet(IWin32Window owner, ReleaseNotes notes)
        {
            using (WhatsNewForm f = new WhatsNewForm(notes))
                return f.ShowDialog(owner) == DialogResult.OK;
        }

        public WhatsNewForm(ReleaseNotes notes)
        {
            this.notes = notes;

            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            FormBorderStyle = FormBorderStyle.None;
            ShowInTaskbar = false;
            StartPosition = FormStartPosition.CenterParent;
            BackColor = T.Card;
            KeyPreview = true;
            try { Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath); }
            catch { }

            ClientSize = new Size(T.P(640), T.P(600));

            text = new TextBox();
            text.Multiline = true;
            text.ReadOnly = true;
            text.BorderStyle = BorderStyle.None;
            text.ScrollBars = ScrollBars.Vertical;
            text.WordWrap = true;
            text.TabStop = false;
            text.BackColor = T.Card;
            text.ForeColor = T.Text;
            text.Font = T.FBody;
            text.Text = NotesText.ToPlain(notes.Body)
                .Replace("\r\n", "\n").Replace('\r', '\n').Replace("\n", "\r\n");
            Controls.Add(text);

            openBtn = new FlatBtn();
            openBtn.Label = L.T("Open release", "Release öffnen");
            openBtn.Icon = "download";
            openBtn.Primary = true;
            openBtn.Click += delegate { DialogResult = DialogResult.OK; Close(); };
            Controls.Add(openBtn);

            closeBtn = new FlatBtn();
            closeBtn.Label = L.T("Close", "Schließen");
            closeBtn.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(closeBtn);

            close = new WinBtn();
            close.Icon = "close";
            close.BackColor = T.Card;
            close.Click += delegate { DialogResult = DialogResult.Cancel; Close(); };
            Controls.Add(close);

            LayoutAll();
            SheetFade.Attach(this);
        }

        void LayoutAll()
        {
            if (text == null || openBtn == null || closeBtn == null || close == null) return;
            int pad = T.P(24);
            int w = ClientSize.Width, h = ClientSize.Height;
            rHead = new Rectangle(0, 0, w, T.P(66));
            rFoot = new Rectangle(0, h - T.P(64), w, T.P(64));
            close.SetBounds(w - T.P(10) - T.P(42), T.P(8), T.P(42), T.P(30));

            rText = new Rectangle(pad, rHead.Bottom + T.P(14), w - pad * 2,
                Math.Max(T.P(80), rFoot.Top - rHead.Bottom - T.P(24)));
            text.SetBounds(rText.X, rText.Y, rText.Width, rText.Height);

            int bh = T.P(36);
            int by = rFoot.Y + (rFoot.Height - bh) / 2;
            openBtn.SetBounds(w - pad - T.P(140), by, T.P(140), bh);
            closeBtn.SetBounds(openBtn.Left - T.P(8) - T.P(96), by, T.P(96), bh);
        }

        [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)]
        static extern int SetWindowTheme(IntPtr hWnd, string subApp, string subIdList);

        protected override void OnHandleCreated(EventArgs e)
        {
            base.OnHandleCreated(e);
            try { SetWindowTheme(text.Handle, "DarkMode_Explorer", null); } catch { }
        }

        protected override void OnShown(EventArgs e)
        {
            base.OnShown(e);
            ApplyWindowShape();
        }

        protected override void OnResize(EventArgs e)
        {
            base.OnResize(e);
            ApplyWindowShape();
            LayoutAll();
        }

        void ApplyWindowShape()
        {
            if (ClientSize.Width <= 0 || ClientSize.Height <= 0) return;
            using (GraphicsPath p = T.Round(new RectangleF(0, 0, Width, Height), T.F(12f)))
                Region = new Region(p);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            g.SmoothingMode = SmoothingMode.AntiAlias;
            T.PaintSheet(g, ClientRectangle, BackColor);

            int pad = T.P(24);
            RectangleF tile = new RectangleF(pad, T.P(16), T.P(34), T.P(34));
            T.FillRound(g, tile, T.F(8f), Color.FromArgb(38, T.Gold.R, T.Gold.G, T.Gold.B));
            T.StrokeRound(g, tile, T.F(8f), Color.FromArgb(92, T.Gold.R, T.Gold.G, T.Gold.B), 1f);
            float ic = T.F(19f);
            T.Icon(g, "info", new RectangleF(tile.X + (tile.Width - ic) / 2f,
                tile.Y + (tile.Height - ic) / 2f, ic, ic), T.Gold);

            int titleX = pad + T.P(34) + T.P(12);
            T.Str(g, L.T("What's new", "Neuigkeiten"), T.FCardTitle, T.Text,
                new Rectangle(titleX, T.P(12), Width - titleX - T.P(56), T.P(28)), T.FL);
            T.StrEll(g, notes.Name, T.FSmall, T.Muted,
                new Rectangle(titleX, T.P(40), Width - titleX - T.P(56), T.P(18)), T.FL);

            T.HLine(g, 0, Width, rHead.Bottom, T.LineSoft);
            T.HLine(g, 0, Width, rFoot.Y, T.LineSoft);

            using (GraphicsPath rim = T.Round(new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f), T.F(12f)))
            using (Pen pen = new Pen(T.Stroke, 1f))
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
            if (keyData == Keys.Escape)
            {
                DialogResult = DialogResult.Cancel;
                Close();
                return true;
            }
            return base.ProcessCmdKey(ref msg, keyData);
        }
    }
}
