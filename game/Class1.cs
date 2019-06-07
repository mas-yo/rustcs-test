using System;
using System.Linq;
using System.Runtime.InteropServices;
using System.Threading;

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
            var sw = new System.Diagnostics.Stopwatch();
            sw.Start();
            for(int i = 0;i < 1000000;i++) {
                OnReceiveCS(30,40);
            }
            sw.Stop();
            TimeSpan ts = sw.Elapsed;
            Console.WriteLine("time {0}", (int)ts.TotalMilliseconds);
        }

        public delegate void SendFn([In,Out] ObjInfo[] objinfo, int size);
        public static void OnReceive(int x, int y, SendFn sendfn)
        {
            ObjInfo[] data = new ObjInfo[50];
            for(int i = 0; i < 50; i++) {
                data[i].Name = "aaaaa";
                data[i].X = x;
                data[i].Y = y;
            }

            sendfn(data, 50);
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
