interface my_bus_if #(parameter ADDR_W = 32, DATA_W = 64) (
    input logic clk,
    input logic rst_n
);

    logic [ADDR_W-1:0] addr;
    logic [DATA_W-1:0] wdata;
    logic [DATA_W-1:0] rdata;
    logic              valid;
    logic              ready;

    modport master (
        output addr, wdata, valid,
        input  rdata, ready,
        input  clk, rst_n
    );

    modport slave (
        input  addr, wdata, valid,
        output rdata, ready,
        input  clk, rst_n
    );

    clocking cb @(posedge clk);
        default input #1 output #1;
        input  addr, wdata, valid;
        output rdata, ready;
    endclocking

endinterface
