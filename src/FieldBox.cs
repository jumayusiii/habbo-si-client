using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Windows.Forms;

namespace HabboSiClient
{
    /// <summary>Themed single-line input with placeholder support.</summary>
    internal class FieldBox : Control
    {
        public readonly TextBox Box;
        public string Placeholder = "";
        bool hot, focused, showingPh;

        public FieldBox()
        {
            SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                     ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
            BackColor = T.Card;

            Box = new TextBox();
            Box.BorderStyle = BorderStyle.None;
            Box.BackColor = T.Inset;
            Box.ForeColor = T.Text;
            Box.Font = T.FInput;
            Box.TabStop = true;
            Box.GotFocus += delegate
            {
                focused = true;
                if (showingPh) { showingPh = false; Box.Text = ""; Box.ForeColor = T.Text; }
                Invalidate();
            };
            Box.LostFocus += delegate
            {
                focused = false;
                if (Box.Text.Length == 0) ShowPlaceholder();
                Invalidate();
            };
            Box.TextChanged += delegate { if (ValueChanged != null) ValueChanged(); };
            Controls.Add(Box);
        }

        public event Action ValueChanged;

        void ShowPlaceholder()
        {
            if (string.IsNullOrEmpty(Placeholder)) return;
            showingPh = true;
            Box.Text = Placeholder;
            Box.ForeColor = T.Dim;
        }

        public void SetPlaceholder(string text)
        {
            Placeholder = text;
            if (Box.Text.Length == 0 || showingPh) ShowPlaceholder();
        }

        /// <summary>Repaints the embedded text box in the active palette.</summary>
        public void ApplyTheme()
        {
            BackColor = T.Card;
            Box.BackColor = T.Inset;
            Box.ForeColor = showingPh ? T.Dim : T.Text;
            Invalidate();
        }

        public string Value
        {
            get { return showingPh ? "" : Box.Text; }
            set
            {
                if (string.IsNullOrEmpty(value)) { showingPh = false; Box.Text = ""; ShowPlaceholder(); }
                else { showingPh = false; Box.Text = value; Box.ForeColor = T.Text; }
            }
        }

        protected override void OnResize(EventArgs e)
        {
            base.OnResize(e);
            LayoutBox();
        }

        void LayoutBox()
        {
            int pad = T.P(13);
            Box.Width = Math.Max(10, Width - pad * 2);
            Box.Left = pad;
            Box.Top = (Height - Box.Height) / 2;
        }

        protected override void OnMouseEnter(EventArgs e) { hot = true; Invalidate(); base.OnMouseEnter(e); }
        protected override void OnMouseLeave(EventArgs e) { hot = false; Invalidate(); base.OnMouseLeave(e); }
        protected override void OnMouseDown(MouseEventArgs e) { Box.Focus(); base.OnMouseDown(e); }

        protected override void OnPaint(PaintEventArgs e)
        {
            Graphics g = e.Graphics;
            using (SolidBrush bg = new SolidBrush(BackColor)) g.FillRectangle(bg, ClientRectangle);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            RectangleF r = new RectangleF(0.5f, 0.5f, Width - 1f, Height - 1f);
            Color border = focused ? T.Focus : (hot ? T.StrokeHot : T.Stroke);
            T.FillRound(g, r, T.F(8f), T.Inset);
            T.StrokeRound(g, r, T.F(8f), border, focused ? 1.4f : 1f);
        }
    }
}
