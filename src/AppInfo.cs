namespace HabboSiClient
{
    /// <summary>Name, version and the one hotel this client is built for.</summary>
    internal static class AppInfo
    {
#if BETA
        /// <summary>Beta builds are labelled, profiled and updated separately from stable.</summary>
        public const bool Beta = true;
        public const string Name = "Habbo.si Client Beta";
        public const string Version = "1.1.5-beta.1";
        public const string DataFolder = "HabboSiClientBeta";
#else
        public const bool Beta = false;
        public const string Name = "Habbo.si Client";
        public const string Version = "1.1.5";
        public const string DataFolder = "HabboSiClient";
#endif

        /// <summary>Date of the build that ships with this version, shown on the header pill.</summary>
        public const string BuildDate = "2026-11-08";

        /// <summary>The page the client opens: the hotel's own client route.</summary>
        public const string ClientUrl = "https://habbo.si/client";
        public const string SiteUrl = "https://habbo.si";
    }
}
