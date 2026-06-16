module params #(
    parameter ADDR_WIDTH = 32,
    parameter DATA_WIDTH = 64,
    localparam STRB_WIDTH = DATA_WIDTH / 8
)(
    input  wire [ADDR_WIDTH-1:0] addr,
    input  wire [DATA_WIDTH-1:0] wdata,
    output wire [DATA_WIDTH-1:0] rdata,
    input  wire                  wen
);

    reg [DATA_WIDTH-1:0] mem [0:2**ADDR_WIDTH-1];

    assign rdata = mem[addr];

    always @(posedge wen) begin
        mem[addr] <= wdata;
    end

endmodule
