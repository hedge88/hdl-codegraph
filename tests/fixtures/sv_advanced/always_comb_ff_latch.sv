module always_kinds (
    input  logic       clk,
    input  logic       rst_n,
    input  logic [3:0] sel,
    input  logic [7:0] a,
    input  logic [7:0] b,
    output logic [7:0] mux_out,
    output logic [7:0] reg_out,
    output logic [7:0] lat_out
);

    // Combinational: always_comb
    always_comb begin
        case (sel)
            4'd0:    mux_out = a;
            4'd1:    mux_out = b;
            4'd2:    mux_out = a & b;
            4'd3:    mux_out = a | b;
            default: mux_out = 8'b0;
        endcase
    end

    // Sequential: always_ff
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            reg_out <= 8'b0;
        else
            reg_out <= mux_out;
    end

    // Latch: always_latch
    always_latch begin
        if (sel[0])
            lat_out = a;
        if (sel[1])
            lat_out = b;
    end

    // Sequential (plain Verilog): always @(posedge clk)
    reg [7:0] verilog_reg;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            verilog_reg <= 8'b0;
        else
            verilog_reg <= a;
    end

endmodule
