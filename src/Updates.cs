using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Text;

namespace HabboSiClient
{
    /// <summary>The newest release for this channel, as read from the GitHub releases feed.</summary>
    internal sealed class UpdateInfo
    {
        /// <summary>Numeric version without a leading "v" or a suffix, e.g. "1.1.6".</summary>
        public string Latest = "";
        /// <summary>Full tag without the leading "v", e.g. "1.1.5-beta.2".</summary>
        public string Tag = "";
        public string PageUrl = "";

        /// <summary>What the update sheet names: the beta tag, or the numeric stable version.</summary>
        public string Display { get { return Tag.Length > 0 ? Tag : Latest; } }
    }

    /// <summary>
    /// One quiet lookup per launch: the latest stable release for stable builds, and the newest
    /// pre-release for beta builds. It only reads a tag and a URL — nothing is downloaded,
    /// installed or uploaded, and a failure simply means no update prompt.
    /// </summary>
    internal static class UpdateCheck
    {
        const string ApiRoot = "https://api.github.com/repos/jumayusiii/habbo-si-client/";
        const string LatestReleaseUrl = ApiRoot + "releases/latest";
        const string ReleasesUrl = ApiRoot + "releases?per_page=30";

        public static UpdateInfo Latest()
        {
#if BETA
            return LatestBeta();
#else
            return LatestStable();
#endif
        }

        static UpdateInfo LatestStable()
        {
            string json = Get(LatestReleaseUrl);
            if (json == null) return null;
            string tag = Field(json, "tag_name");
            string page = Field(json, "html_url");
            if (tag.Length == 0 || page.Length == 0) return null;
            string numeric = CleanTag(tag);
            return new UpdateInfo { Latest = numeric, Tag = numeric, PageUrl = page };
        }

        /// <summary>The releases list includes pre-releases; /releases/latest deliberately does not.</summary>
        static UpdateInfo LatestBeta()
        {
            string json = Get(ReleasesUrl);
            if (json == null) return null;
            foreach (string release in Objects(json))
            {
                if (BoolField(release, "draft") || !BoolField(release, "prerelease")) continue;
                string tag = Field(release, "tag_name");
                string page = Field(release, "html_url");
                if (tag.Length == 0 || page.Length == 0) continue;
                return new UpdateInfo
                {
                    Latest = CleanTag(tag),
                    Tag = NormalizeTag(tag),
                    PageUrl = page
                };
            }
            return null;
        }

        /// <summary>True when this channel's newest release is ahead of the running build.</summary>
        public static bool HasUpdate(UpdateInfo info)
        {
            if (info == null) return false;
#if BETA
            return IsNewerBeta(info.Tag);
#else
            return IsNewer(info.Latest);
#endif
        }

        static string Get(string url)
        {
            try
            {
                // GitHub's API only answers over TLS 1.2+; older Windows defaults may not try it.
                ServicePointManager.SecurityProtocol |= SecurityProtocolType.Tls12;

                HttpWebRequest request = (HttpWebRequest)WebRequest.Create(url);
                request.Method = "GET";
                request.UserAgent = "HabboSiClient/" + AppInfo.Version;
                request.Accept = "application/vnd.github+json";
                request.Timeout = 6000;
                request.ReadWriteTimeout = 6000;
                request.AutomaticDecompression = DecompressionMethods.GZip | DecompressionMethods.Deflate;

                using (HttpWebResponse response = (HttpWebResponse)request.GetResponse())
                using (StreamReader reader = new StreamReader(response.GetResponseStream(), Encoding.UTF8))
                {
                    return reader.ReadToEnd();
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
            string s = NormalizeTag(tag);
            int dash = s.IndexOf('-');
            if (dash > 0) s = s.Substring(0, dash);
            return s.Trim();
        }

        /// <summary>Drops only the leading "v" so a beta suffix survives.</summary>
        static string NormalizeTag(string tag)
        {
            if (string.IsNullOrEmpty(tag)) return "";
            string s = tag.Trim();
            if (s.StartsWith("v", StringComparison.OrdinalIgnoreCase)) s = s.Substring(1);
            return s;
        }

        /// <summary>True when the release tag names a higher stable version than the running build.</summary>
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

        /// <summary>
        /// Beta channel comparison: version first, then the beta ordinal. "1.1.5-beta.2" beats
        /// "1.1.5-beta.1", and "1.1.6-beta.1" beats both.
        /// </summary>
        public static bool IsNewerBeta(string latest, string running = null)
        {
            string a = NormalizeTag(latest);
            string b = NormalizeTag(running ?? AppInfo.Version);
            if (a.Length == 0) return false;
            if (string.Equals(a, b, StringComparison.OrdinalIgnoreCase)) return false;

            int[] pa = Parts(a);
            int[] pb = Parts(b);
            for (int i = 0; i < 4; i++)
            {
                if (pa[i] != pb[i]) return pa[i] > pb[i];
            }
            return Ordinal(a) > Ordinal(b);
        }

        static int Ordinal(string version)
        {
            int dash = version.IndexOf('-');
            if (dash < 0 || dash == version.Length - 1) return 0;
            string suffix = version.Substring(dash + 1).ToLowerInvariant();
            int dot = suffix.LastIndexOf('.');
            if (dot >= 0) suffix = suffix.Substring(dot + 1);
            int n;
            return int.TryParse(suffix, NumberStyles.Integer, CultureInfo.InvariantCulture, out n) ? n : 0;
        }

        static int[] Parts(string version)
        {
            int[] p = new int[4];
            string s = version ?? "";
            int dash = s.IndexOf('-');
            if (dash > 0) s = s.Substring(0, dash);
            string[] bits = s.Split('.');
            for (int i = 0; i < bits.Length && i < 4; i++)
                int.TryParse(bits[i], NumberStyles.Integer, CultureInfo.InvariantCulture, out p[i]);
            return p;
        }

        /// <summary>Reads a top-level boolean field; GitHub pretty-prints with a space after the colon.</summary>
        static bool BoolField(string json, string key)
        {
            int i = json.IndexOf("\"" + key + "\"", StringComparison.Ordinal);
            if (i < 0) return false;
            i = json.IndexOf(':', i + key.Length + 2);
            if (i < 0) return false;
            i++;
            while (i < json.Length && char.IsWhiteSpace(json[i])) i++;
            return i + 4 <= json.Length &&
                string.Compare(json, i, "true", 0, 4, StringComparison.OrdinalIgnoreCase) == 0;
        }

        /// <summary>Splits the releases array into its top-level objects without a JSON dependency.</summary>
        static IEnumerable<string> Objects(string json)
        {
            int depth = 0, start = -1;
            bool inString = false, escape = false;
            for (int i = 0; i < json.Length; i++)
            {
                char c = json[i];
                if (inString)
                {
                    if (escape) escape = false;
                    else if (c == '\\') escape = true;
                    else if (c == '"') inString = false;
                    continue;
                }
                if (c == '"') { inString = true; continue; }
                if (c == '{')
                {
                    if (depth == 0) start = i;
                    depth++;
                }
                else if (c == '}')
                {
                    if (depth == 0) continue;
                    depth--;
                    if (depth == 0 && start >= 0)
                    {
                        yield return json.Substring(start, i - start + 1);
                        start = -1;
                    }
                }
            }
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
