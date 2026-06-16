module counter #(parameter WIDTH = 8) (
    input  wire              clk,
    input  wire              rst_n,
    output reg  [WIDTH-1:0]  count
);

    wire [WIDTH-1:0] next_count;

    assign next_count = count + 1'b1;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            count <= {WIDTH{1'b0}};
        else
            count <= next_count;
    end

endmodule
