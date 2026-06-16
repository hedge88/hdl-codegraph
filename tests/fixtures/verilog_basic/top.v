module top (
    input  wire       clk,
    input  wire       rst_n,
    input  wire [7:0] a,
    input  wire [7:0] b,
    output wire [8:0] sum,
    output wire [7:0] count
);

    wire [7:0] counter_next;

    counter #(.WIDTH(8)) u_counter (
        .clk   (clk),
        .rst_n (rst_n),
        .count (count)
    );

    adder #(.DATA_WIDTH(8)) u_adder (
        .a   (a),
        .b   (b),
        .sum (sum)
    );

endmodule
