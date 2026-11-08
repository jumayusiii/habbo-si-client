using System;
using System.Globalization;
using System.IO;
using System.Net;
using System.Text;

namespace HabboSiClient
{
    /// <summary>The newest public release, as read from the GitHub releases feed.</summary>
    internal sealed class UpdateInfo
    {
        /// <summary>Version without the tag's leading "v" or a "-hotfix" suffix.</summary>
        public string Latest = "";
        public string PageUrl = "";
    }

    /// <summary>
    /// One quiet lookup per launch: the public "latest release" endpoint for the client repo.
    /// It only reads a tag and a URL — nothing is downloaded, installed or uploaded, and a
    /// failure (offline, rate limit, API change) simply means no update prompt.
    /// </summary>
    internal static class UpdateCheck
    {
        const string LatestReleaseUrl =
            "https://api.github.com/repos/jumayusiii/habbo-si-client/releases/latest";

        public static UpdateInfo Latest()
        {
            try
            {
                // GitHub's API only answers over TLS 1.2+; older Windows defaults may not try it.
                ServicePointManager.SecurityProtocol |= SecurityProtocolType.Tls12;

                HttpWebRequest request = (HttpWebRequest)WebRequest.Create(LatestReleaseUrl);
                request.Method = "GET";
                request.UserAgent = "HabboSiClient/" + AppInfo.Version;
                request.Accept = "application/vnd.github+json";
                request.Timeout = 6000;
                request.ReadWriteTimeout = 6000;
                request.AutomaticDecompression = DecompressionMethods.GZip | DecompressionMethods.Deflate;

                using (HttpWebResponse response = (HttpWebResponse)request.GetResponse())
                using (StreamReader reader = new StreamReader(response.GetResponseStream(), Encoding.UTF8))
                {
                    string json = reader.ReadToEnd();
                    string tag = Field(json, "tag_name");
                    string page = Field(json, "html_url");
                    if (tag.Length == 0 || page.Length == 0) return null;
                    return new UpdateInfo { Latest = CleanTag(tag), PageUrl = page };
                }
            }
            catch
            {
                return null;
            }
        }

        /// <summary>"v1.1.6" and "v1.1.5-hotfix" become "1.1.6" and "1.1.5".</summary>
        public static string CleanTag(string tag)
        {
            if (string.IsNullOrEmpty(tag)) return "";
            string s = tag.Trim();
            if (s.StartsWith("v", StringComparison.OrdinalIgnoreCase)) s = s.Substring(1);
            int dash = s.IndexOf('-');
            if (dash > 0) s = s.Substring(0, dash);
            return s.Trim();
        }

        /// <summary>True when the release tag names a higher version than the running build.</summary>
        public static bool IsNewer(string latest, string running = null)
        {
            int[] a = Parts(latest);
            int[] b = Parts(running ?? AppInfo.Version);
            for (int i = 0; i < 4; i++)
            {
                if (a[i] != b[i]) return a[i] > b[i];
            }
            return false;
        }

        static int[] Parts(string version)
        {
            int[] p = new int[4];
            string[] bits = (version ?? "").Split('.');
            for (int i = 0; i < bits.Length && i < 4; i++)
                int.TryParse(bits[i], NumberStyles.Integer, CultureInfo.InvariantCulture, out p[i]);
            return p;
        }

        /// <summary>Reads a top-level string field; GitHub pretty-prints with a space after the colon.</summary>
        static string Field(string json, string key)
        {
            int i = json.IndexOf("\"" + key + "\"", StringComparison.Ordinal);
            if (i < 0) return "";
            i = json.IndexOf(':', i + key.Length + 2);
            if (i < 0) return "";
            i = json.IndexOf('"', i + 1);
            if (i < 0) return "";

            StringBuilder sb = new StringBuilder();
            for (i++; i < json.Length; i++)
            {
                char c = json[i];
                if (c == '\\' && i + 1 < json.Length) { sb.Append(json[++i]); continue; }
                if (c == '"') break;
                sb.Append(c);
            }
            return sb.ToString().Trim();
        }
    }
}
