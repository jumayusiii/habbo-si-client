using System.Text;
using Microsoft.Web.WebView2.Core;

namespace HabboSiClient
{
    /// <summary>
    /// The Chromium switches the client starts its renderer with. They are deliberately a
    /// short, documented list rather than a grab bag: keep the GPU path on, and stop the
    /// browser from treating a window that is covered by another window as idle.
    /// </summary>
    internal static class Edge
    {
        public static string Arguments(ClientSettings s)
        {
            StringBuilder a = new StringBuilder();
            if (s.Gpu)
            {
                a.Append("--enable-gpu-rasterization ");
                a.Append("--enable-zero-copy ");
                a.Append("--ignore-gpu-blocklist ");
            }
            if (s.NoThrottle)
            {
                // a hotel client keeps its socket and timers alive while it is not the
                // foreground window; Chromium throttles both by default
                a.Append("--disable-background-timer-throttling ");
                a.Append("--disable-renderer-backgrounding ");
                a.Append("--disable-backgrounding-occluded-windows ");
                a.Append("--disable-features=CalculateNativeWinOcclusion ");
            }
            // hotel sounds and the client's own music should start without a click first
            a.Append("--autoplay-policy=no-user-gesture-required ");
            return a.ToString().Trim();
        }

        public static CoreWebView2EnvironmentOptions Options(ClientSettings s)
        {
            return new CoreWebView2EnvironmentOptions(Arguments(s));
        }
    }
}
