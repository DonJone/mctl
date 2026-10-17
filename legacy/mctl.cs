using System;
using System.IO;
using System.IO.Compression;
using System.Net.Sockets;
using System.Text;
using System.Diagnostics;
using System.Threading;
using System.Collections.Generic;
using System.Globalization;

namespace MinecraftControl
{
    class Program
    {
        static string ServerDir = @"E:\MinecraftServer";
        static string LogFile = @"E:\MinecraftServer\logs\latest.log";
        static string TaskName = "MinecraftServer";
        static int RconPort = 25575;
        static string RconPass = "";
        static int ServerPort = 25565;
        const string VERSION = "mctl 1.2.0";
        const string AUTHOR = "DonJone";
        const string AUTHOR_URL = "https://github.com/DonJone";

        static void InitConfig()
        {
            string envDir = Environment.GetEnvironmentVariable("MCTL_DIR");
            if (!string.IsNullOrEmpty(envDir) && Directory.Exists(envDir))
            {
                ServerDir = envDir;
            }
            else if (File.Exists("server.properties"))
            {
                ServerDir = Directory.GetCurrentDirectory();
            }
            else if (Directory.Exists(@"E:\MinecraftServer"))
            {
                ServerDir = @"E:\MinecraftServer";
            }
            else if (Directory.Exists(@"C:\MinecraftServer"))
            {
                ServerDir = @"C:\MinecraftServer";
            }

            LogFile = Path.Combine(ServerDir, @"logs\latest.log");

            string propFile = Path.Combine(ServerDir, "server.properties");
            if (File.Exists(propFile))
            {
                try
                {
                    foreach (string rawLine in File.ReadAllLines(propFile))
                    {
                        string line = rawLine.Trim();
                        if (line.StartsWith("#") || !line.Contains("=")) continue;
                        int eqIdx = line.IndexOf('=');
                        string key = line.Substring(0, eqIdx).Trim();
                        string val = line.Substring(eqIdx + 1).Trim();

                        int parsedPort;
                        if (key.Equals("rcon.port", StringComparison.OrdinalIgnoreCase) && int.TryParse(val, out parsedPort))
                        {
                            RconPort = parsedPort;
                        }
                        else if (key.Equals("rcon.password", StringComparison.OrdinalIgnoreCase))
                        {
                            RconPass = val;
                        }
                        else if (key.Equals("server-port", StringComparison.OrdinalIgnoreCase) && int.TryParse(val, out parsedPort))
                        {
                            ServerPort = parsedPort;
                        }
                    }
                }
                catch { }
            }

            string envTask = Environment.GetEnvironmentVariable("MCTL_TASK");
            if (!string.IsNullOrEmpty(envTask)) TaskName = envTask;

            string envRconPass = Environment.GetEnvironmentVariable("MCTL_RCON_PASS");
            if (!string.IsNullOrEmpty(envRconPass)) RconPass = envRconPass;

            string envRconPort = Environment.GetEnvironmentVariable("MCTL_RCON_PORT");
            int envPortVal;
            if (!string.IsNullOrEmpty(envRconPort) && int.TryParse(envRconPort, out envPortVal)) RconPort = envPortVal;
        }

        static bool IsChinese = true;

        static string T(string zh, string en)
        {
            return IsChinese ? zh : en;
        }

        static void DetectLanguage(ref string[] args)
        {
            // 1. Detect from OS Culture
            string uiName = CultureInfo.CurrentUICulture.Name;
            string sysName = CultureInfo.InstalledUICulture.Name;
            IsChinese = uiName.StartsWith("zh", StringComparison.OrdinalIgnoreCase) ||
                        sysName.StartsWith("zh", StringComparison.OrdinalIgnoreCase);

            // 2. Detect from Environment Variable MCTL_LANG
            string envLang = Environment.GetEnvironmentVariable("MCTL_LANG");
            if (!string.IsNullOrEmpty(envLang))
            {
                IsChinese = envLang.StartsWith("zh", StringComparison.OrdinalIgnoreCase);
            }

            // 3. Detect from command line options (-l <lang>, --lang <lang>, --lang=<lang>)
            var cleanArgs = new List<string>();
            for (int i = 0; i < args.Length; i++)
            {
                string arg = args[i];
                if (arg.Equals("-l", StringComparison.OrdinalIgnoreCase) || arg.Equals("--lang", StringComparison.OrdinalIgnoreCase))
                {
                    if (i + 1 < args.Length)
                    {
                        string val = args[i + 1];
                        IsChinese = val.StartsWith("zh", StringComparison.OrdinalIgnoreCase);
                        i++;
                    }
                }
                else if (arg.StartsWith("--lang=", StringComparison.OrdinalIgnoreCase))
                {
                    string val = arg.Substring(7);
                    IsChinese = val.StartsWith("zh", StringComparison.OrdinalIgnoreCase);
                }
                else if (arg.Equals("--zh", StringComparison.OrdinalIgnoreCase))
                {
                    IsChinese = true;
                }
                else if (arg.Equals("--en", StringComparison.OrdinalIgnoreCase))
                {
                    IsChinese = false;
                }
                else
                {
                    cleanArgs.Add(arg);
                }
            }
            args = cleanArgs.ToArray();
        }

        static int Main(string[] args)
        {
            Console.OutputEncoding = Encoding.UTF8;
            DetectLanguage(ref args);
            InitConfig();

            if (args.Length == 0)
            {
                PrintHelp();
                return 0;
            }

            string command = args[0].ToLowerInvariant();

            try
            {
                switch (command)
                {
                    case "start":
                        return HandleStart();
                    case "stop":
                        return HandleStop();
                    case "restart":
                        return HandleRestart();
                    case "status":
                        return HandleStatus();
                    case "rcon":
                        return HandleRcon(args);
                    case "log":
                    case "logs":
                        return HandleLog(args);
                    case "backup":
                        return HandleBackup();
                    case "reload":
                        return HandleReload();
                    case "license":
                    case "--license":
                        PrintLicense();
                        return 0;
                    case "-v":
                    case "--version":
                    case "version":
                        PrintVersion();
                        return 0;
                    case "-h":
                    case "--help":
                    case "help":
                        PrintHelp();
                        return 0;
                    default:
                        Console.Error.WriteLine(T("mctl: 未知指令 '{0}'", "mctl: unknown command '{0}'"), args[0]);
                        Console.Error.WriteLine(T("运行 'mctl --help' 获取帮助。", "Try 'mctl --help' for more information."));
                        return 2;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(T("mctl: 错误: {0}", "mctl: error: {0}"), ex.Message);
                return 1;
            }
        }

        static int HandleStart()
        {
            Process proc = GetServerProcess();
            if (proc != null)
            {
                Console.WriteLine(T("服务已在运行中 (PID: {0})", "Service is already running (PID: {0})"), proc.Id);
                return 0;
            }

            Console.WriteLine(T("正在启动服务...", "Starting service..."));
            RunCommand("schtasks.exe", string.Format("/run /tn \"{0}\"", TaskName));

            for (int i = 0; i < 40; i++)
            {
                Thread.Sleep(1000);
                if (IsPortListening(ServerPort))
                {
                    Process p = GetServerProcess();
                    Console.WriteLine(T("服务启动成功 (PID: {0}, 端口: {1})", "Service started successfully (PID: {0}, Port: {1})"),
                        p != null ? p.Id.ToString() : "unknown", ServerPort);
                    return 0;
                }
            }

            Console.Error.WriteLine(T("警告: 端口尚未就绪，可运行 'mctl log' 查看日志。", "Warning: port not listening yet. Run 'mctl log' to inspect."));
            return 1;
        }

        static int HandleStop()
        {
            Process proc = GetServerProcess();
            if (proc == null)
            {
                Console.WriteLine(T("服务已处于停止状态。", "Service is already stopped."));
                return 0;
            }

            Console.WriteLine(T("正在停止服务 (PID: {0})...", "Stopping service (PID: {0})..."), proc.Id);
            try
            {
                SendRconCommand("save-all");
                SendRconCommand("stop");
            }
            catch
            {
                RunCommand("schtasks.exe", string.Format("/end /tn \"{0}\"", TaskName));
            }

            for (int i = 0; i < 25; i++)
            {
                Thread.Sleep(1000);
                if (GetServerProcess() == null)
                {
                    Console.WriteLine(T("服务已停止。", "Service stopped."));
                    return 0;
                }
            }

            proc = GetServerProcess();
            if (proc != null)
            {
                try { proc.Kill(); } catch { }
            }
            Console.WriteLine(T("服务进程已终止。", "Service process terminated."));
            return 0;
        }

        static int HandleRestart()
        {
            Console.WriteLine(T("正在重启服务...", "Restarting service..."));
            HandleStop();
            Thread.Sleep(2000);
            return HandleStart();
        }

        static int HandleStatus()
        {
            Process proc = GetServerProcess();
            bool portOpen = IsPortListening(ServerPort);

            Console.WriteLine(T("服务名称:   {0}", "Service:    {0}"), TaskName);
            Console.WriteLine(T("运行路径:   {0}", "Directory:  {0}"), ServerDir);

            if (proc == null)
            {
                Console.WriteLine(T("运行状态:   inactive (已停止)", "Active:     inactive (dead)"));
                Console.WriteLine(T("端口监听:   {0} (未监听)", "Port {0}:   closed"), ServerPort);
                return 3; // Standard Unix status code for inactive service
            }

            Console.WriteLine(T("运行状态:   active (运行中) 起始于 {0:yyyy-MM-dd HH:mm:ss}", "Active:     active (running) since {0:yyyy-MM-dd HH:mm:ss}"), proc.StartTime);
            Console.WriteLine(T("主进程号:   {0}", "Main PID:   {0}"), proc.Id);

            try
            {
                long memBytes = proc.WorkingSet64;
                double memMB = memBytes / (1024.0 * 1024.0);
                double memGB = memMB / 1024.0;
                Console.WriteLine(T("内存占用:   {0:F1} MB ({1:F2} GB)", "Memory:     {0:F1} MB ({1:F2} GB)"), memMB, memGB);
                Console.WriteLine(T("CPU 时间:   {0:hh\\:mm\\:ss}", "CPU Time:   {0:hh\\:mm\\:ss}"), proc.TotalProcessorTime);
            }
            catch { }

            Console.WriteLine(T("端口监听:   {0} ({1})", "Port {0}:   {1}"), ServerPort, portOpen ? T("监听中", "listening (open)") : T("未监听", "not listening"));

            try
            {
                string rconList = SendRconCommand("list");
                Console.WriteLine(T("在线玩家:   {0}", "Players:    {0}"), rconList.Trim());
            }
            catch
            {
                Console.WriteLine(T("在线玩家:   [RCON 启动中]", "Players:    [RCON warming up]"));
            }

            return 0;
        }

        static int HandleRcon(string[] args)
        {
            if (args.Length < 2)
            {
                Console.Error.WriteLine(T("用法: mctl rcon <指令...>", "Usage: mctl rcon <command...>"));
                return 2;
            }

            string fullCmd = string.Join(" ", args, 1, args.Length - 1);
            string response = SendRconCommand(fullCmd);
            Console.WriteLine(response.Trim());
            return 0;
        }

        static int HandleReload()
        {
            Console.WriteLine(T("正在重载数据包与配置...", "Reloading datapacks and configs..."));
            string resp = SendRconCommand("reload");
            Console.WriteLine(resp.Trim());
            return 0;
        }

        static int HandleLog(string[] args)
        {
            int lines = 30;
            bool follow = false;
            int n;

            for (int i = 1; i < args.Length; i++)
            {
                if (args[i] == "-f" || args[i] == "--follow")
                {
                    follow = true;
                }
                else if (args[i] == "-n" || args[i] == "--lines")
                {
                    if (i + 1 < args.Length && int.TryParse(args[i + 1], out n))
                    {
                        lines = n;
                        i++;
                    }
                }
                else if (int.TryParse(args[i], out n))
                {
                    lines = n;
                }
            }

            if (!File.Exists(LogFile))
            {
                Console.Error.WriteLine(T("未找到日志文件: '{0}'", "Log file not found at '{0}'"), LogFile);
                return 1;
            }

            // Print last N lines
            var queue = new Queue<string>(lines);
            using (var fs = new FileStream(LogFile, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            using (var reader = new StreamReader(fs, Encoding.UTF8))
            {
                string line;
                while ((line = reader.ReadLine()) != null)
                {
                    if (queue.Count >= lines) queue.Dequeue();
                    queue.Enqueue(line);
                }
            }

            foreach (var l in queue)
            {
                Console.WriteLine(l);
            }

            if (!follow) return 0;

            // Follow mode
            using (var fs = new FileStream(LogFile, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            {
                fs.Seek(0, SeekOrigin.End);
                using (var reader = new StreamReader(fs, Encoding.UTF8))
                {
                    while (true)
                    {
                        string line = reader.ReadLine();
                        if (line != null)
                        {
                            Console.WriteLine(line);
                        }
                        else
                        {
                            Thread.Sleep(250);
                        }
                    }
                }
            }
        }

        static int HandleBackup()
        {
            string worldDir = Path.Combine(ServerDir, "world");
            string backupDir = Path.Combine(Path.GetDirectoryName(ServerDir) ?? ServerDir, "MC_Backups");
            string envBackupDir = Environment.GetEnvironmentVariable("MCTL_BACKUP_DIR");
            if (!string.IsNullOrEmpty(envBackupDir)) backupDir = envBackupDir;

            if (!Directory.Exists(worldDir))
            {
                Console.Error.WriteLine(T("未找到世界存档目录: '{0}'", "World directory not found: '{0}'"), worldDir);
                return 1;
            }

            if (!Directory.Exists(backupDir))
            {
                Directory.CreateDirectory(backupDir);
            }

            Console.WriteLine(T("正在同步世界数据...", "Syncing world data..."));
            try
            {
                SendRconCommand("save-all");
                Thread.Sleep(2000);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(T("警告: 同步世界数据异常 ({0})", "Warning: sync world data failed ({0})"), ex.Message);
            }

            string timestamp = DateTime.Now.ToString("yyyyMMdd_HHmmss");
            string zipFile = Path.Combine(backupDir, string.Format("world_{0}.zip", timestamp));

            Console.WriteLine(T("正在创建备份归档: {0}...", "Archiving world data: {0}..."), Path.GetFileName(zipFile));
            using (var archive = ZipFile.Open(zipFile, ZipArchiveMode.Create))
            {
                foreach (var filePath in Directory.GetFiles(worldDir, "*", SearchOption.AllDirectories))
                {
                    string fileName = Path.GetFileName(filePath);
                    if (fileName.Equals("session.lock", StringComparison.OrdinalIgnoreCase))
                        continue;

                    string relativePath = filePath.Substring(worldDir.Length).TrimStart('\\', '/');
                    try
                    {
                        using (var srcStream = new FileStream(filePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
                        {
                            var entry = archive.CreateEntry(relativePath, CompressionLevel.Optimal);
                            using (var destStream = entry.Open())
                            {
                                srcStream.CopyTo(destStream);
                            }
                        }
                    }
                    catch (Exception ex)
                    {
                        Console.WriteLine(T("跳过锁定文件 '{0}': {1}", "Skipping locked file '{0}': {1}"), relativePath, ex.Message);
                    }
                }
            }

            var fileInfo = new FileInfo(zipFile);
            double sizeMB = fileInfo.Length / (1024.0 * 1024.0);
            Console.WriteLine(T("备份完成: {0} ({1:F2} MB)", "Backup created: {0} ({1:F2} MB)"), Path.GetFileName(zipFile), sizeMB);

            // Clean older than 7 days
            try
            {
                DateTime cutoff = DateTime.Now.AddDays(-7);
                foreach (var oldFile in Directory.GetFiles(backupDir, "*.zip"))
                {
                    if (File.GetLastWriteTime(oldFile) < cutoff)
                    {
                        File.Delete(oldFile);
                        Console.WriteLine(T("已清理过期备份: {0}", "Pruned expired backup: {0}"), Path.GetFileName(oldFile));
                    }
                }
            }
            catch { }

            return 0;
        }

        static Process GetServerProcess()
        {
            foreach (var p in Process.GetProcessesByName("java"))
            {
                try
                {
                    string path = p.MainModule.FileName;
                    if (path.IndexOf("Java21", StringComparison.OrdinalIgnoreCase) >= 0 ||
                        path.IndexOf("MinecraftServer", StringComparison.OrdinalIgnoreCase) >= 0 ||
                        path.IndexOf("java-runtime-delta", StringComparison.OrdinalIgnoreCase) >= 0)
                    {
                        return p;
                    }
                }
                catch { }
            }
            return null;
        }

        static bool IsPortListening(int port)
        {
            try
            {
                using (var client = new TcpClient(AddressFamily.InterNetworkV6))
                {
                    var res = client.BeginConnect("::1", port, null, null);
                    if (res.AsyncWaitHandle.WaitOne(500))
                    {
                        client.EndConnect(res);
                        return true;
                    }
                }
            }
            catch { }

            try
            {
                using (var client = new TcpClient())
                {
                    var res = client.BeginConnect("127.0.0.1", port, null, null);
                    if (res.AsyncWaitHandle.WaitOne(500))
                    {
                        client.EndConnect(res);
                        return true;
                    }
                }
            }
            catch { }

            return false;
        }

        static string SendRconCommand(string command)
        {
            if (string.IsNullOrEmpty(RconPass))
            {
                throw new Exception(T("未配置 RCON 口令。请在 server.properties 中配置 rcon.password 或设置 MCTL_RCON_PASS 环境变量。", "RCON password not configured. Set rcon.password in server.properties or MCTL_RCON_PASS env var."));
            }

            using (var client = ConnectRcon())
            using (var stream = client.GetStream())
            {
                // Auth
                byte[] authPayload = Encoding.UTF8.GetBytes(RconPass);
                SendRconPacket(stream, 1, 3, authPayload);
                var authResp = ReadRconPacket(stream);
                if (authResp.RequestId == -1)
                {
                    throw new Exception(T("RCON 认证失败 (口令错误)。", "RCON authentication failed (invalid password)."));
                }

                // Command
                byte[] cmdPayload = Encoding.UTF8.GetBytes(command);
                SendRconPacket(stream, 2, 2, cmdPayload);
                var cmdResp = ReadRconPacket(stream);
                return Encoding.UTF8.GetString(cmdResp.Payload);
            }
        }

        static TcpClient ConnectRcon()
        {
            string[] hosts = new string[] { "::1", "127.0.0.1" };
            foreach (var h in hosts)
            {
                try
                {
                    var c = new TcpClient(h.IndexOf(":") >= 0 ? AddressFamily.InterNetworkV6 : AddressFamily.InterNetwork);
                    c.SendTimeout = 4000;
                    c.ReceiveTimeout = 4000;
                    var ar = c.BeginConnect(h, RconPort, null, null);
                    if (ar.AsyncWaitHandle.WaitOne(2000))
                    {
                        c.EndConnect(ar);
                        return c;
                    }
                    c.Close();
                }
                catch { }
            }
            throw new Exception(T(string.Format("无法连接至 RCON 端口 {0} (::1 或 127.0.0.1)", RconPort), string.Format("Cannot connect to RCON port {0} on ::1 or 127.0.0.1", RconPort)));
        }

        struct RconResponse
        {
            public int RequestId;
            public int Type;
            public byte[] Payload;
        }

        static void SendRconPacket(NetworkStream stream, int reqId, int type, byte[] payload)
        {
            int len = 10 + payload.Length;
            byte[] packet = new byte[4 + len];
            BitConverter.GetBytes(len).CopyTo(packet, 0);
            BitConverter.GetBytes(reqId).CopyTo(packet, 4);
            BitConverter.GetBytes(type).CopyTo(packet, 8);
            payload.CopyTo(packet, 12);
            packet[packet.Length - 2] = 0;
            packet[packet.Length - 1] = 0;
            stream.Write(packet, 0, packet.Length);
            stream.Flush();
        }

        static RconResponse ReadRconPacket(NetworkStream stream)
        {
            byte[] header = new byte[12];
            int read = 0;
            while (read < 12)
            {
                int r = stream.Read(header, read, 12 - read);
                if (r <= 0) throw new Exception(T("RCON 连接已被服务器关闭。", "RCON connection closed by server."));
                read += r;
            }
            int len = BitConverter.ToInt32(header, 0);
            int reqId = BitConverter.ToInt32(header, 4);
            int type = BitConverter.ToInt32(header, 8);
            int bodyLen = len - 8;
            byte[] body = new byte[bodyLen];
            read = 0;
            while (read < bodyLen)
            {
                int r = stream.Read(body, read, bodyLen - read);
                if (r <= 0) throw new Exception(T("读取 RCON 数据包时连接异常中断。", "RCON connection closed by server during body."));
                read += r;
            }
            int payloadLen = Math.Max(0, bodyLen - 2);
            byte[] payload = new byte[payloadLen];
            if (payloadLen > 0)
            {
                Array.Copy(body, 0, payload, 0, payloadLen);
            }
            return new RconResponse { RequestId = reqId, Type = type, Payload = payload };
        }

        static void RunCommand(string file, string args)
        {
            var psi = new ProcessStartInfo(file, args)
            {
                CreateNoWindow = true,
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true
            };
            using (var p = Process.Start(psi))
            {
                p.WaitForExit(10000);
            }
        }

        static void PrintVersion()
        {
            Console.WriteLine("{0}", VERSION);
            Console.WriteLine("Copyright (C) 2026 DonJone (https://github.com/DonJone)");
            Console.WriteLine(T("许可协议: 非商业伦理许可协议 (运行 'mctl license' 查看全文)", "License:  Ethical Source License (run 'mctl license' for details)"));
        }

        static void PrintLicense()
        {
            if (IsChinese)
            {
                Console.WriteLine("版权所有 (c) 2026 DonJone (https://github.com/DonJone)");
                Console.WriteLine();
                Console.WriteLine("特此向任何获得本软件副本及相关文档（下称“软件”）的人免费授予许可，");
                Console.WriteLine("允许其在不受限制的情况下使用本软件，包括但不限于运行、复制、修改、合并、");
                Console.WriteLine("发布、分发及二次授权，但须严格遵守以下条件：");
                Console.WriteLine();
                Console.WriteLine("1. 上述版权声明及本许可通知必须完整包含在软件的所有副本或实质性部分中。");
                Console.WriteLine();
                Console.WriteLine("2. 商业排他与限制条款：");
                Console.WriteLine("   严禁任何商业实体、盈利性云厂商或托管服务平台在未经原作者书面明确授权的情况下：");
                Console.WriteLine("   (a) 将本软件作为商业产品、付费增值服务或商业控制面板的一部分进行销售或运营；");
                Console.WriteLine("   (b) 将本软件源码或衍生代码用于商业化大语言模型的训练语料；");
                Console.WriteLine("   (c) 闭源分发或移除原作者版权署名。");
                Console.WriteLine("   非商业、个人游玩及社区交流用途不受本条款限制，完全免费。");
                Console.WriteLine();
                Console.WriteLine("3. 免责声明：");
                Console.WriteLine("   本软件按“现状”提供，不包含任何明示或暗示的担保。在任何情况下，作者均不对因使用");
                Console.WriteLine("   本软件所产生的任何直接或间接损失承担法律责任。");
            }
            else
            {
                Console.WriteLine("Copyright (c) 2026 DonJone (https://github.com/DonJone)");
                Console.WriteLine();
                Console.WriteLine("Permission is hereby granted, free of charge, to any person obtaining a copy");
                Console.WriteLine("of this software and associated documentation files (the \"Software\"), to deal");
                Console.WriteLine("in the Software without restriction, including without limitation the rights");
                Console.WriteLine("to use, copy, modify, merge, publish, distribute, and sublicense the Software,");
                Console.WriteLine("subject to the following conditions:");
                Console.WriteLine();
                Console.WriteLine("1. The above copyright notice and this permission notice shall be included in");
                Console.WriteLine("   all copies or substantial portions of the Software.");
                Console.WriteLine();
                Console.WriteLine("2. COMMERCIAL EXCLUSION RESTRICTION:");
                Console.WriteLine("   Use of the Software by any commercial enterprise, hosting provider, or cloud");
                Console.WriteLine("   SaaS/PaaS vendor for revenue-generating purposes, or ingestion into commercial");
                Console.WriteLine("   machine learning training datasets, is prohibited without prior written license");
                Console.WriteLine("   from the author. Personal, community, and non-commercial uses are exempt.");
                Console.WriteLine();
                Console.WriteLine("3. DISCLAIMER OF WARRANTY:");
                Console.WriteLine("   THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR");
                Console.WriteLine("   IMPLIED. IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM OR DAMAGES.");
            }
        }

        static void PrintHelp()
        {
            if (IsChinese)
            {
                Console.WriteLine("用法: mctl <指令> [选项]");
                Console.WriteLine();
                Console.WriteLine("Minecraft 服务端管理工具");
                Console.WriteLine();
                Console.WriteLine("指令:");
                Console.WriteLine("  status              查看服务运行状态与性能指标");
                Console.WriteLine("  start               启动服务端");
                Console.WriteLine("  stop                停止服务端");
                Console.WriteLine("  restart             重启服务端");
                Console.WriteLine("  reload              重载服务端配置与数据包");
                Console.WriteLine("  rcon <指令...>      向服务端发送控制台指令");
                Console.WriteLine("  log [-n 行数] [-f]  查看服务端日志");
                Console.WriteLine("  backup              创建世界存档备份");
                Console.WriteLine("  license             显示软件许可协议");
                Console.WriteLine();
                Console.WriteLine("选项:");
                Console.WriteLine("  -l, --lang <zh|en>  设置显示语言");
                Console.WriteLine("  -v, --version       显示版本信息");
                Console.WriteLine("  -h, --help          显示帮助信息");
            }
            else
            {
                Console.WriteLine("Usage: mctl <command> [options]");
                Console.WriteLine();
                Console.WriteLine("Minecraft Server Controller");
                Console.WriteLine();
                Console.WriteLine("Commands:");
                Console.WriteLine("  status              Display service status and metrics");
                Console.WriteLine("  start               Start the server");
                Console.WriteLine("  stop                Stop the server");
                Console.WriteLine("  restart             Restart the server");
                Console.WriteLine("  reload              Reload server configuration and datapacks");
                Console.WriteLine("  rcon <cmd...>       Execute a console command via RCON");
                Console.WriteLine("  log [-n N] [-f]     View server logs");
                Console.WriteLine("  backup              Create a backup archive of the world");
                Console.WriteLine("  license             Display software license terms");
                Console.WriteLine();
                Console.WriteLine("Options:");
                Console.WriteLine("  -l, --lang <zh|en>  Set display language");
                Console.WriteLine("  -v, --version       Display version information");
                Console.WriteLine("  -h, --help          Show this help message");
            }
        }
    }
}
