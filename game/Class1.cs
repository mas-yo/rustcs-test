using System;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Threading.Tasks;
using System.Threading.Tasks.Dataflow;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Text;
// using Lockfreeq;

namespace game
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public struct ObjInfo
    {
        public string Name;
        public int X;
        public int Y;

        public ObjInfo(string name, int x, int y)
        {
            this.Name = name;
            this.X = x;
            this.Y = y;
        }
    }
    public class Class1
    {
        public static void Main()
        {
            Console.WriteLine("{0}", System.Reflection.Assembly.GetExecutingAssembly().FullName);
            Console.WriteLine("This assembly is not meant to be run directly.");
            Console.WriteLine("Instead, please use the SampleHost process to load this assembly.");

            StartServer();
            Thread.Sleep(1000000);

            // var sw = new System.Diagnostics.Stopwatch();
            // sw.Start();
            // for(int i = 0;i < 1000000;i++) {
            //     OnReceiveCS(30,40);
            // }
            // sw.Stop();
            // TimeSpan ts = sw.Elapsed;
            // Console.WriteLine("time {0}", (int)ts.TotalMilliseconds);
        }

        static async void StartServer()
        {
            TcpListener listener = new TcpListener(29180);
            listener.Start();
            // List<Channel> room_channels = new List<Channel>();
            while (true) {
                // 2. クライアントからの接続を受け入れる
                var socket = await listener.AcceptTcpClientAsync();
                Console.WriteLine("acc");

                Task.Run( async()=>{
                    var writer = new StreamWriter(socket.GetStream(), System.Text.Encoding.ASCII);
                    var reader = new StreamReader(socket.GetStream(), System.Text.Encoding.ASCII);

                    while(true) {
                        var line = await reader.ReadLineAsync();

                        if(line.StartsWith("input_text")) {
                            ObjInfo[] data = new ObjInfo[50];
                            for(int i = 0; i < 50; i++) {
                                data[i].Name = "aaaaa";
                                data[i].X = 1;
                                data[i].Y = 2;
                            }

                            StringBuilder sb = new StringBuilder();
                            sb.Append("objlist,");
                            foreach( var d in data ) {
                                sb.AppendFormat("{0}:{1},{2}/", "testname", 1, 2);
                            }
                            // Console.WriteLine("msg {0}", line);
                            await writer.WriteLineAsync(sb.ToString());
                            await writer.FlushAsync();
                        }
                    }
                        // var data = await socket.ReceiveAsync(SocketAsyncEventArgs.Empty);
                });




                // BufferBlock<TcpClient> channel = null;
                // if(room_channels.Count == 0) {
                //     channel = NewRoom();
                //     room_channels.Add(new Channel(channel,1));
                //     Console.WriteLine("new room {0}", room_channels.Count);
                // }
                // else if(room_channels[room_channels.Count-1].member_count < 10) {
                //     channel = room_channels[room_channels.Count-1].channel;
                //     room_channels[room_channels.Count-1].member_count++;
                //     Console.WriteLine("new member {0}", room_channels[room_channels.Count-1].member_count);
                // }
                // else {
                //     channel = NewRoom();
                //     room_channels.Add(new Channel(channel,1));
                //     Console.WriteLine("new room {0}", room_channels.Count);
                // }
                // await channel.SendAsync(client);
            }
        }


        public delegate void SendFn([In,Out] ObjInfo[] objinfo, int size);

        public static SendFn sendFn;
        public static void OnReceive(int x, int y)//, SendFn sendfn)
        {
            ObjInfo[] data = new ObjInfo[50];
            for(int i = 0; i < 50; i++) {
                data[i].Name = "aaaaa";
                data[i].X = x;
                data[i].Y = y;
            }

            sendFn(data, 50);
        }

        public static void SetSendFn(SendFn f)
        {
            Console.WriteLine("cs - SetSendFn");
            sendFn = f;
        }

        public static void SendCS(ObjInfo[] objinfo)
        {
            foreach(var obj in objinfo){
                // Console.Write("{0}", obj.X);
            }
        }
        public static void OnReceiveCS(int x, int y)
        {
            ObjInfo[] data = new ObjInfo[50];
            for(int i = 0; i < 50; i++) {
                data[i].Name = "aaaaa";
                data[i].X = x;
                data[i].Y = y;
            }
            SendCS(data);
        }



        public delegate int ReportProgressFunction(int progress);

        // This test method doesn't actually do anything, it just takes some input parameters,
        // waits (in a loop) for a bit, invoking the callback function periodically, and
        // then returns a string version of the double[] passed in.
        [return: MarshalAs(UnmanagedType.LPStr)]
        public static string DoWork(
            [MarshalAs(UnmanagedType.LPStr)] string jobName,
            int iterations,
            int dataSize,
            [MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 2)] double[] data,
            ReportProgressFunction reportProgressFunction)
        {

            Console.WriteLine(jobName);
            for (int i = 1; i <= iterations; i++)
            {
                Console.ForegroundColor = ConsoleColor.Cyan;
                Console.WriteLine($"Beginning work iteration {i}");
                Console.ResetColor();

                // Pause as if doing work
                Thread.Sleep(1000);

                // Call the native callback and write its return value to the console
                var progressResponse = reportProgressFunction(i);
                Console.WriteLine($"Received response [{progressResponse}] from progress function");
            }

            Console.ForegroundColor = ConsoleColor.Green;
            Console.WriteLine($"Work completed");
            Console.ResetColor();

            return $"Data received: {string.Join(", ", data.Select(d => d.ToString()))}";
        }
    }
}
